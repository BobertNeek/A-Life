//! CPU/ECS parity with the pre-optimization selection pass, without a renderer.
use super::*;

type HighlightSnapshotRow = (String, String, Option<(String, String)>);

#[derive(Resource, Default)]
struct ChangedMeshes(Vec<Entity>);

fn record_changed_meshes(
    meshes: Query<Entity, Changed<MeshMaterial3d<StandardMaterial>>>,
    mut changed: ResMut<ChangedMeshes>,
) {
    changed.0 = meshes.iter().collect();
}

// Frozen from main 03a9e249, god_hand.rs blob b162cff20aeed53b96e835058e2cb71a9164f6fa.
#[allow(clippy::too_many_arguments)]
fn highlight_frozen(
    mut commands: Commands,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    children: Query<&Children>,
    roots: Query<(Entity, &ProductionCreatureAssemblyRoot)>,
    objects: Query<(Entity, &live_food_projection::LiveCareObject)>,
    mut meshes: Query<(
        Entity,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&SelectedTint>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut markers: Query<&mut Visibility, With<Fvr03ProductionVoxelSelectionMarker>>,
) {
    for mut v in &mut markers {
        *v = Visibility::Hidden;
    }
    let selected = selection.selected.and_then(|s| s.stable_id);
    let target = roots
        .iter()
        .find(|(_, r)| Some(r.stable_id) == selected)
        .map(|(e, _)| e)
        .or_else(|| {
            objects
                .iter()
                .find(|(_, o)| Some(o.0) == selected)
                .map(|(e, _)| e)
        });
    let descendants: Vec<_> = target
        .into_iter()
        .flat_map(|root| std::iter::once(root).chain(children.iter_descendants(root)))
        .collect();
    for (entity, mut handle, tint) in &mut meshes {
        if descendants.contains(&entity) {
            if let Some(tint) = tint {
                handle.0 = tint.highlighted.clone();
            } else if let Some(source) = materials.get(&handle.0) {
                let mut highlighted = source.clone();
                highlighted.emissive = bevy::color::LinearRgba::rgb(0.12, 0.095, 0.025);
                let highlighted = materials.add(highlighted);
                let original = handle.0.clone();
                handle.0 = highlighted.clone();
                commands.entity(entity).insert(SelectedTint {
                    original,
                    highlighted,
                });
            }
        } else if let Some(tint) = tint {
            handle.0 = tint.original.clone();
        }
    }
}

struct Fixture {
    app: App,
    root: Entity,
    other: Entity,
    child: Entity,
    material: Handle<StandardMaterial>,
    replacement: Handle<StandardMaterial>,
}

impl Fixture {
    fn new(optimized: bool, unrelated: usize) -> Self {
        let mut app = App::new();
        app.insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(Fvr03ProductionVoxelSelectionResource {
                hovered: None,
                selected: None,
            })
            .init_resource::<ChangedMeshes>();
        if optimized {
            app.add_systems(Update, (highlight, record_changed_meshes.after(highlight)));
        } else {
            app.add_systems(
                Update,
                (
                    highlight_frozen,
                    record_changed_meshes.after(highlight_frozen),
                ),
            );
        }
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.4, 0.6),
                metallic: 0.25,
                perceptual_roughness: 0.37,
                emissive: bevy::color::LinearRgba::rgb(0.01, 0.02, 0.03),
                ..default()
            });
        let replacement = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.3, 0.1),
                unlit: true,
                ..default()
            });
        let root = app
            .world_mut()
            .spawn((
                ProductionCreatureAssemblyRoot {
                    stable_id: WorldEntityId(1),
                    organism_id: OrganismId(11),
                    display_only: true,
                },
                Name::new("root"),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        let other = app
            .world_mut()
            .spawn((
                live_food_projection::LiveCareObject(WorldEntityId(2)),
                Name::new("other"),
                MeshMaterial3d(replacement.clone()),
            ))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Name::new("child"),
                ChildOf(root),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        for index in 0..unrelated {
            app.world_mut().spawn((
                Name::new(format!("unrelated-{index}")),
                MeshMaterial3d(material.clone()),
            ));
        }
        app.world_mut()
            .spawn((Fvr03ProductionVoxelSelectionMarker, Visibility::Visible));
        Self {
            app,
            root,
            other,
            child,
            material,
            replacement,
        }
    }

    fn select(&mut self, stable_id: Option<u64>) {
        self.app
            .world_mut()
            .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
            .selected = stable_id.map(|id| StableVoxelObjectRef {
            kind: StableVoxelRefKind::Creature,
            stable_id: Some(WorldEntityId(id)),
            tile: None,
            chunk: VoxelChunkCoord::new(0, 0),
        });
    }

    fn handle(&self, entity: Entity) -> &Handle<StandardMaterial> {
        &self
            .app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .unwrap()
            .0
    }

    fn changed(&self) -> usize {
        self.app.world().resource::<ChangedMeshes>().0.len()
    }

    fn update(&mut self) {
        self.app.update();
    }

    fn snapshot(&mut self) -> Vec<HighlightSnapshotRow> {
        let world = self.app.world_mut();
        let mut query = world.query::<(
            &Name,
            &MeshMaterial3d<StandardMaterial>,
            Option<&SelectedTint>,
        )>();
        let assets = world.resource::<Assets<StandardMaterial>>();
        let mut result = query
            .iter(world)
            .map(|(name, handle, tint)| {
                let value = |handle: &Handle<StandardMaterial>| format!("{:?}", assets.get(handle));
                (
                    name.to_string(),
                    value(&handle.0),
                    tint.map(|tint| (value(&tint.original), value(&tint.highlighted))),
                )
            })
            .collect::<Vec<_>>();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

#[test]
fn stable_selection_and_deselection_stop_material_writes_among_3716_props() {
    let mut old = Fixture::new(false, 3716);
    let mut new = Fixture::new(true, 3716);
    for fixture in [&mut old, &mut new] {
        fixture.select(Some(1));
        fixture.update();
        assert_ne!(fixture.handle(fixture.root), &fixture.material);
        assert_ne!(fixture.handle(fixture.child), &fixture.material);
        assert_eq!(fixture.handle(fixture.other), &fixture.replacement);
    }
    assert_eq!(old.snapshot(), new.snapshot());
    old.update();
    new.update();
    assert_eq!(old.changed(), 2);
    assert_eq!(new.changed(), 0);
    assert_eq!(old.snapshot(), new.snapshot());
    for fixture in [&mut old, &mut new] {
        fixture.select(Some(2));
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &fixture.material);
        assert_eq!(fixture.handle(fixture.child), &fixture.material);
        assert_ne!(fixture.handle(fixture.other), &fixture.replacement);
        fixture.select(None);
        fixture.update();
        assert_eq!(fixture.handle(fixture.other), &fixture.replacement);
        fixture.update();
    }
    assert_eq!(old.changed(), 3);
    assert_eq!(new.changed(), 0);
    assert_eq!(old.snapshot(), new.snapshot());
    assert_eq!(
        new.app.world().resource::<Assets<StandardMaterial>>().len(),
        5
    );
}

#[test]
fn fresh_hierarchy_and_cached_material_semantics_match_the_frozen_pass() {
    let run = |optimized| {
        let mut fixture = Fixture::new(optimized, 8);
        let mut states = Vec::new();
        fixture.select(Some(1));
        fixture.update();
        states.push(fixture.snapshot());
        // Late glTF descendant with a material and an existing node gaining one.
        let late = fixture
            .app
            .world_mut()
            .spawn((Name::new("late"), ChildOf(fixture.root)))
            .id();
        fixture.update();
        fixture
            .app
            .world_mut()
            .entity_mut(late)
            .insert(MeshMaterial3d(fixture.replacement.clone()));
        fixture.update();
        states.push(fixture.snapshot());
        // Leaving the selected subtree restores a previously tinted material.
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.child)
            .insert(ChildOf(fixture.other));
        fixture.update();
        assert_eq!(fixture.handle(fixture.child), &fixture.material);
        states.push(fixture.snapshot());
        // Existing SelectedTint deliberately retains the first original handle.
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.root)
            .insert(MeshMaterial3d(fixture.replacement.clone()));
        fixture.update();
        states.push(fixture.snapshot());
        fixture.select(None);
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &fixture.material);
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.root)
            .insert(MeshMaterial3d(fixture.replacement.clone()));
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &fixture.material);
        states.push(fixture.snapshot());
        // Same selected stable ID, newly created root after the old root despawns.
        fixture.select(Some(1));
        fixture.app.world_mut().despawn(fixture.root);
        let root = fixture
            .app
            .world_mut()
            .spawn((
                ProductionCreatureAssemblyRoot {
                    stable_id: WorldEntityId(1),
                    organism_id: OrganismId(12),
                    display_only: true,
                },
                Name::new("replacement-root"),
                MeshMaterial3d(fixture.replacement.clone()),
            ))
            .id();
        fixture.update();
        assert_ne!(fixture.handle(root), &fixture.replacement);
        states.push(fixture.snapshot());
        states
    };
    assert_eq!(run(false), run(true));
}

#[test]
fn missing_source_retries_and_creature_root_keeps_target_priority() {
    for optimized in [false, true] {
        let mut fixture = Fixture::new(optimized, 0);
        let missing = fixture
            .app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .reserve_handle();
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.root)
            .insert(MeshMaterial3d(missing.clone()));
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.other)
            .insert(live_food_projection::LiveCareObject(WorldEntityId(1)));
        fixture.select(Some(1));
        fixture.update();
        assert!(fixture
            .app
            .world()
            .get::<SelectedTint>(fixture.root)
            .is_none());
        assert_eq!(fixture.handle(fixture.other), &fixture.replacement);
        fixture
            .app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(missing.id(), StandardMaterial::default())
            .unwrap();
        fixture.update();
        assert!(fixture
            .app
            .world()
            .get::<SelectedTint>(fixture.root)
            .is_some());
        assert_ne!(fixture.handle(fixture.root), &missing);
        assert_eq!(fixture.handle(fixture.other), &fixture.replacement);
    }
}

#[test]
fn asset_edits_and_removed_material_components_preserve_cached_behavior() {
    let run = |optimized| {
        let mut fixture = Fixture::new(optimized, 0);
        let mut states = Vec::new();
        fixture.select(Some(1));
        fixture.update();
        let highlighted = fixture.handle(fixture.root).clone();
        fixture
            .app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .get_mut(&fixture.material)
            .unwrap()
            .base_color = Color::srgb(0.9, 0.2, 0.1);
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &highlighted);
        states.push(fixture.snapshot());
        fixture.select(None);
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &fixture.material);
        states.push(fixture.snapshot());
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.root)
            .remove::<MeshMaterial3d<StandardMaterial>>();
        fixture.update();
        fixture
            .app
            .world_mut()
            .entity_mut(fixture.root)
            .insert(MeshMaterial3d(fixture.replacement.clone()));
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &fixture.material);
        fixture.select(Some(1));
        fixture.update();
        assert_eq!(fixture.handle(fixture.root), &highlighted);
        states.push(fixture.snapshot());
        states
    };
    assert_eq!(run(false), run(true));
}
