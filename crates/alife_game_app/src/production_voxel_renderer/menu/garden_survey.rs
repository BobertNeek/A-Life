//! A map of the player's planted beds. The survey only selects a place; the
//! normal Hand and world still decide whether sowing or picking can happen.
use super::*;
use alife_world::orchard::OrchardStage;
mod chart;

#[derive(Default, Resource)]
pub(super) struct Survey {
    plots: Vec<(VoxelTileCoord, OrchardStage)>,
    index: usize,
    soil: Option<VoxelTileCoord>,
    #[cfg(feature = "gpu-runtime")]
    arrival: Option<Destination>,
}
#[cfg(feature = "gpu-runtime")]
#[derive(Clone, Copy)]
enum Destination {
    Bed(VoxelTileCoord),
    Soil(VoxelTileCoord),
}
#[cfg(feature = "gpu-runtime")]
impl Destination {
    fn resolve(self, world: &alife_world::HeadlessWorld) -> Option<VoxelTileCoord> {
        use alife_world::orchard::OrchardGesture;
        let tile = match self {
            Self::Bed(t) | Self::Soil(t) => t,
        };
        let gesture = world.orchard_gesture(tile)?;
        (matches!(self, Self::Soil(_)) == (gesture == OrchardGesture::Sow)).then_some(tile)
    }
}
impl Survey {
    fn selected(&self) -> Option<(VoxelTileCoord, OrchardStage)> {
        self.plots.get(self.index).copied()
    }
    pub(super) fn available(&self, action: Action) -> bool {
        match action {
            Action::PreviousBed | Action::NextBed => self.plots.len() > 1,
            Action::VisitBed => self.selected().is_some(),
            Action::FindSoil => self.soil.is_some(),
            _ => true,
        }
    }
    fn read(
        &mut self,
        world: &alife_world::HeadlessWorld,
        preferred: Option<VoxelTileCoord>,
        action: Action,
    ) {
        self.plots = world
            .ecology()
            .orchard_beds
            .iter()
            .map(|b| (b.tile, b.stage(world.tick())))
            .collect();
        self.index = preferred
            .and_then(|t| self.plots.iter().position(|(tile, _)| *tile == t))
            .unwrap_or(0);
        if !self.plots.is_empty() {
            self.index = match action {
                Action::NextBed => (self.index + 1) % self.plots.len(),
                Action::PreviousBed => (self.index + self.plots.len() - 1) % self.plots.len(),
                _ => self.index,
            };
        }
    }
}
#[derive(Resource)]
pub(super) struct Specimens([Handle<Image>; 4]);
impl Specimens {
    pub(super) fn load(world: &mut World) -> Self {
        let images = [
            include_bytes!("../../../../../assets/ui/garden_survey/seed.png").as_slice(),
            include_bytes!("../../../../../assets/ui/garden_survey/bud.png"),
            include_bytes!("../../../../../assets/ui/garden_survey/bloom.png"),
            include_bytes!("../../../../../assets/ui/garden_survey/ripe.png"),
        ];
        Self(images.map(|png| care_dock::decode_png(world, png)))
    }
    fn get(&self, stage: OrchardStage) -> Handle<Image> {
        self.0[match stage {
            OrchardStage::Seed => 0,
            OrchardStage::Bud => 1,
            OrchardStage::Bloom => 2,
            OrchardStage::Ripe => 3,
        }]
        .clone()
    }
}
#[derive(Component)]
pub(super) struct Relief;
#[derive(Component)]
struct MapImage;
#[derive(Component)]
struct Specimen;

pub(super) fn mount(parent: &mut ChildSpawner, seed: &Handle<Image>) {
    parent
        .spawn((
            Relief,
            accessible(Role::Image, "Orchard survey"),
            Node {
                width: Val::Percent(100.),
                aspect_ratio: Some(16. / 9.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                border: UiRect::all(Val::Px(3.)),
                ..default()
            },
            BackgroundColor(ink::WELL),
            BorderColor::all(ink::PORCELAIN),
            bevy::picking::Pickable::IGNORE,
        ))
        .with_children(|mount| {
            mount.spawn((
                Name::new("Orchard terrain survey"),
                MapImage,
                ImageNode::default(),
                Node {
                    width: Val::Percent(74.),
                    aspect_ratio: Some(4. / 3.),
                    ..default()
                },
                FocusPolicy::Block,
            ));
            mount
                .spawn((
                    Node {
                        width: Val::Percent(23.),
                        aspect_ratio: Some(1.),
                        border: UiRect::all(Val::Px(2.)),
                        border_radius: BorderRadius::all(Val::Percent(50.)),
                        ..default()
                    },
                    BackgroundColor(ink::GLASS),
                    bevel(true),
                    bevy::picking::Pickable::IGNORE,
                ))
                .with_children(|rim| {
                    rim.spawn((
                        Specimen,
                        ImageNode::new(seed.clone()),
                        Node {
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            ..default()
                        },
                        bevy::picking::Pickable::IGNORE,
                    ));
                });
        });
}

#[cfg(feature = "gpu-runtime")]
fn arrive(world: &mut World, destination: Destination) {
    let Some(tile) = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .and_then(|r| destination.resolve(r.runtime.world()))
    else {
        return;
    };
    world
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = Some(orchard::selected_soil(tile));
    world
        .resource_mut::<Fvr04ProductionCreatureFollowResource>()
        .enabled = false;
    world.resource_mut::<care_dock::State>().hand_focus = Some(care_dock::HandFocus::bed(tile));
}

fn pointed_bed(world: &mut World) -> Option<VoxelTileCoord> {
    if world.resource::<MenuState>().release_gate
        || !world
            .get_resource::<ButtonInput<MouseButton>>()?
            .just_pressed(MouseButton::Left)
    {
        return None;
    }
    let cursor = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .ok()?
        .physical_cursor_position()?;
    let (node, pose) = world
        .query_filtered::<(&ComputedNode, &bevy::ui::UiGlobalTransform), With<MapImage>>()
        .single(world)
        .ok()?;
    if !node.contains_point(*pose, cursor) {
        return None;
    }
    let normalized = pose.affine().inverse().transform_point2(cursor) / node.size();
    chart::pick(&world.get_resource::<Survey>()?.plots, normalized)
}

pub(super) fn handle(world: &mut World, outcome: &mut InputOutcome, focused: bool) {
    let screen = world.resource::<MenuState>().screen;
    #[cfg(feature = "gpu-runtime")]
    let captured = world.resource::<MenuState>().captured;
    let action = world.resource_mut::<MenuState>().garden_action.take();
    #[cfg(feature = "gpu-runtime")]
    if let Some(mut state) = world.get_resource_mut::<Survey>() {
        if !focused || screen != Screen::Closed {
            state.arrival = None;
        } else if !captured {
            if let Some(destination) = state.arrival.take() {
                arrive(world, destination);
            }
        }
    }
    if screen != Screen::Garden || !focused {
        return;
    }
    // Choosing a mark only turns the lens. Visiting still takes an explicit press.
    let pointed = action.is_none().then(|| pointed_bed(world)).flatten();
    let Some(action) = action.or(pointed.map(|_| Action::Garden)) else {
        return;
    };
    if !world.contains_resource::<Survey>() {
        return;
    }
    world.resource_scope(|world, mut s: Mut<Survey>| {
        #[cfg(feature = "gpu-runtime")]
        let live = world
            .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
            .map(|r| r.runtime.world());
        #[cfg(not(feature = "gpu-runtime"))]
        let live = None::<&alife_world::HeadlessWorld>;
        // Validate the displayed destination, never silently substitute another bed.
        #[cfg(feature = "gpu-runtime")]
        if let Some(destination) = match action {
            Action::VisitBed => s.selected().map(|r| Destination::Bed(r.0)),
            Action::FindSoil => s.soil.map(Destination::Soil),
            _ => None,
        }
        .filter(|d| live.is_some_and(|w| d.resolve(w).is_some()))
        {
            s.arrival = Some(destination);
            outcome.resume = Some(world.resource_mut::<MenuState>().close(false));
            return;
        }
        #[cfg(not(feature = "gpu-runtime"))]
        let _ = &outcome;
        let picked = world
            .get_resource::<Fvr03ProductionVoxelSelectionResource>()
            .and_then(|s| s.selected);
        let preferred = if action == Action::Garden {
            pointed.or_else(|| picked.and_then(|r| r.tile))
        } else {
            s.selected().map(|r| r.0)
        };
        if let Some(live) = live {
            s.read(live, preferred, action);
            #[cfg(feature = "gpu-runtime")]
            {
                s.soil = orchard::starting_soil(
                    live,
                    s.selected().map(|r| orchard::selected_soil(r.0)).or(picked),
                );
            }
        } else {
            *s = Survey::default();
        }
        let drawn = chart::draw(
            live.filter(|_| !s.plots.is_empty())
                .and_then(|w| w.terrain())
                .map(|t| t.surface().as_ref()),
            &s.plots,
            s.selected().map(|r| r.0),
        );
        display(world, &s, drawn);
    });
}

fn display(world: &mut World, s: &Survey, drawn: Image) {
    let art = world.resource_mut::<Assets<Image>>().add(drawn);
    let mut label = "No planted beds".to_owned();
    let specimen = if let Some((t, stage)) = s.selected() {
        label = format!(
            "Bed {} of {} at {}, {}. {}",
            s.index + 1,
            s.plots.len(),
            t.x,
            t.z,
            match stage {
                OrchardStage::Seed => "Seed",
                OrchardStage::Bud => "Budding",
                OrchardStage::Bloom => "Flowering",
                OrchardStage::Ripe => "Ripe pears",
            }
        );
        world.resource::<Specimens>().get(stage)
    } else {
        world.resource::<MenuGlyphs>().sow.clone()
    };
    // The engine derives ImageNode labels from visible text children. Keep the
    // instrument's spoken description on its mount, outside that image pass.
    for mut a in world
        .query_filtered::<&mut AccessibilityNode, With<Relief>>()
        .iter_mut(world)
    {
        a.set_label(label.clone());
    }
    for mut image in world
        .query_filtered::<&mut ImageNode, With<MapImage>>()
        .iter_mut(world)
    {
        image.image = art.clone();
    }
    for mut image in world
        .query_filtered::<&mut ImageNode, With<Specimen>>()
        .iter_mut(world)
    {
        image.image = specimen.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_world::{orchard::OrchardReceipt, HeadlessWorld};

    #[test]
    fn repeating_enter_cannot_select_another_bed_or_leave_the_map() {
        for (row, &(action, _)) in GARDEN.iter().enumerate() {
            let mut state = MenuState {
                screen: Screen::Garden,
                selected: row,
                ..default()
            };
            let mut keys = ButtonInput::default();
            keys.press(KeyCode::Enter);
            state.input(
                &keys,
                false,
                false,
                false,
                None,
                false,
                &[],
                true,
                false,
                LoadAvailability::Ready,
            );
            assert_eq!(state.screen, Screen::Garden);
            assert!(state.garden_action.is_none());
            state.input(
                &keys,
                false,
                false,
                true,
                None,
                false,
                &[],
                true,
                false,
                LoadAvailability::Ready,
            );
            if action == Action::Back {
                assert_eq!(state.screen, Screen::Main);
            } else {
                assert_eq!(state.garden_action, Some(action));
            }
        }
    }
    #[test]
    fn a_saved_empty_household_can_revisit_every_real_bed_without_changing_it() {
        let mut garden = HeadlessWorld::new(43);
        let east = VoxelTileCoord::new(14, -4);
        let west = VoxelTileCoord::new(-7, 3);
        let mut survey = Survey::default();
        survey.read(&garden, None, Action::Garden);
        assert!(!survey.available(Action::VisitBed));
        assert_eq!(garden.tend_orchard(east).unwrap(), OrchardReceipt::Sown);
        for _ in 0..440 {
            garden.try_advance_tick().unwrap();
        }
        assert_eq!(garden.tend_orchard(west).unwrap(), OrchardReceipt::Sown);
        let garden = alife_world::PortableSaveFile::from_headless_world(
            "orchard-survey",
            &garden,
            alife_world::RuntimeConfig::deterministic_default(
                garden.seed(),
                alife_core::BrainScaleTier::Nano512,
            ),
            alife_world::AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap()
        .restore_headless_world()
        .unwrap();
        let before = garden.canonical_signature_digest().unwrap();
        survey.read(&garden, Some(west), Action::Garden);
        assert_eq!(survey.selected(), Some((west, OrchardStage::Seed)));
        survey.read(&garden, Some(west), Action::PreviousBed);
        assert_eq!(survey.selected(), Some((east, OrchardStage::Ripe)));
        survey.read(&garden, Some(east), Action::NextBed);
        assert_eq!(survey.selected(), Some((west, OrchardStage::Seed)));
        #[cfg(feature = "gpu-runtime")]
        assert_eq!(
            garden.orchard_gesture(
                orchard::starting_soil(&garden, Some(orchard::selected_soil(west))).unwrap()
            ),
            Some(alife_world::orchard::OrchardGesture::Sow)
        );
        assert_eq!(garden.canonical_signature_digest().unwrap(), before);
        assert_eq!(garden.object_count(), 0);
        #[cfg(feature = "gpu-runtime")]
        {
            assert_eq!(Destination::Bed(west).resolve(&garden), Some(west));
            assert_eq!(Destination::Soil(west).resolve(&garden), None);
            assert_eq!(
                Destination::Bed(VoxelTileCoord::new(5, 5)).resolve(&garden),
                None
            );
        }
    }
}
