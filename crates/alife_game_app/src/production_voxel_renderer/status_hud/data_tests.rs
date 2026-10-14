use super::*;
use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};
use alife_world::{HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord};
fn authoritative_world() -> HeadlessWorld {
    authoritative_world_with_energy(None)
}
fn authoritative_world_with_energy(energy: Option<f32>) -> HeadlessWorld {
    let id = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(21)
        .agent("agent", id, Vec3f::ZERO)
        .build()
        .unwrap();
    let entity = world.entity_id("agent").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        21,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let mut chemistry =
        alife_core::BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    if let Some(energy) = energy {
        chemistry.body.set_energy(energy).unwrap();
    }
    world
        .register_organism_record(
            WorldOrganismRecord::new(id, entity, genome, phenotype, chemistry, Tick::ZERO).unwrap(),
        )
        .unwrap();
    world
}
fn row() -> WorldOrganismPresentationRow {
    authoritative_world()
        .presentation_snapshot()
        .organisms
        .remove(0)
}
#[test]
fn six_quantities_are_independent_actual_physiology_not_generic_wellbeing() {
    let mut row = row();
    let h = &mut row.biochemistry.homeostasis;
    h.drives.hunger = 0.11;
    h.drives.brain_atp = 0.99;
    h.drives.fatigue = 0.33;
    h.drives.fear = 0.44;
    h.hormones.sleep_pressure = 0.55;
    h.hormones.extension[0] = 0.66;
    h.hormones.dopamine = 0.88;
    h.hormones.oxytocin = 0.77;
    row.biochemistry.body.energy = 0.22;
    let readout = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row));
    assert_eq!(
        readout.values,
        [
            Some(0.11),
            Some(0.22),
            Some(0.33),
            Some(1.0 - 0.44),
            Some(0.55),
            Some(0.66)
        ]
    );
    for quantity in [Quantity::Hunger, Quantity::Fatigue, Quantity::SleepPressure] {
        assert!(!quantity.warning(0.1));
        assert!(quantity.warning(0.9));
    }
    for quantity in [Quantity::Energy, Quantity::Calm] {
        assert!(quantity.warning(0.1));
        assert!(!quantity.warning(0.9));
    }
    assert!(!Quantity::PraiseSignal.warning(0.0));
    assert!(!Quantity::PraiseSignal.warning(1.0));
}
fn set_raw(row: &mut WorldOrganismPresentationRow, quantity: Quantity, value: f32) {
    let h = &mut row.biochemistry.homeostasis;
    match quantity {
        Quantity::Hunger => h.drives.hunger = value,
        Quantity::Energy => row.biochemistry.body.energy = value,
        Quantity::Fatigue => h.drives.fatigue = value,
        Quantity::Calm => h.drives.fear = value,
        Quantity::SleepPressure => h.hormones.sleep_pressure = value,
        Quantity::PraiseSignal => h.hormones.extension[0] = value,
    }
}
#[test]
fn invalid_raw_values_are_unknown_in_each_lane_and_zero_is_known() {
    for quantity in QUANTITIES {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
            let mut row = row();
            set_raw(&mut row, quantity, invalid);
            assert_eq!(quantity.value(&row), None, "{quantity:?}: {invalid}");
        }
        for valid in [0.0, 0.5, 1.0] {
            let mut row = row();
            set_raw(&mut row, quantity, valid);
            assert_eq!(
                quantity.value(&row),
                Some(if quantity == Quantity::Calm {
                    1.0 - valid
                } else {
                    valid
                })
            );
        }
    }
}
#[test]
fn unknown_row_and_every_identity_lifecycle_mismatch_fail_closed() {
    let row = row();
    let stable = row.world_entity_id;
    let organism = row.organism_id;
    assert_eq!(Readout::from_row(stable, organism, None).values, [None; 6]);
    let mut mutations = Vec::new();
    let mut changed = row.clone();
    changed.world_entity_id = WorldEntityId(stable.raw() + 1);
    mutations.push(changed);
    let mut changed = row.clone();
    changed.organism_id = OrganismId(2);
    mutations.push(changed);
    let mut changed = row.clone();
    changed.object.id = WorldEntityId(stable.raw() + 1);
    mutations.push(changed);
    let mut changed = row.clone();
    changed.object.organism_id = None;
    mutations.push(changed);
    let mut changed = row.clone();
    changed.object.organism_id = Some(OrganismId(2));
    mutations.push(changed);
    let mut changed = row.clone();
    changed.object.kind = WorldObjectKind::Food;
    mutations.push(changed);
    let mut changed = row.clone();
    changed.lifecycle = alife_world::OrganismLifecycle::Dead {
        death_tick: Tick(1),
    };
    mutations.push(changed);
    for changed in mutations {
        assert_eq!(
            Readout::from_row(stable, organism, Some(&changed)).values,
            [None; 6]
        );
    }
}
#[test]
fn selected_readout_does_not_depend_on_spawn_metadata_and_rebinds_on_frame_removal() {
    let organism_world = authoritative_world();
    let row = organism_world.presentation_snapshot().organisms.remove(0);
    let stable = row.world_entity_id;
    let mut world = World::new();
    world.init_resource::<BevyEntityMap>();
    assert_eq!(selected_readout(&world, stable).values, [None; 6]);
    let entity = world
        .spawn((
            ProductionCreatureAssemblyRoot {
                stable_id: stable,
                organism_id: row.organism_id,
                display_only: true,
            },
            Fvr04ProductionCreatureVisualMarker {
                stable_id: stable,
                organism_id: row.organism_id,
                tile: VoxelTileCoord { x: 0, z: 0 },
                expression: CreatureExpressionState::Neutral,
                animation: CreatureAnimationState::Idle,
                lod: Fvr04CreatureLod::CompactVoxel,
                base_translation: Vec3::ZERO,
                local_offset: Vec3::ZERO,
                base_scale: Vec3::ONE,
                local_bounds: CreatureVisualBounds::new([-1.0; 3], [1.0; 3]),
                surface_height: 0.0,
                body_yaw: 0.0,
                head_yaw: 0.0,
                phase: 0.0,
            },
        ))
        .id();
    world
        .resource_mut::<BevyEntityMap>()
        .bind(entity, stable)
        .unwrap();
    assert_eq!(selected_readout(&world, stable).values, [None; 6]);
    world.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&organism_world).unwrap(),
    );
    assert!(selected_readout(&world, stable)
        .values
        .iter()
        .all(Option::is_some));
    let measured = selected_readout(&world, stable);
    assert_eq!(
        selected_readout(&world, stable),
        measured,
        "paused readout does not extrapolate or decay"
    );
    world
        .get_mut::<ProductionCreatureAssemblyRoot>(entity)
        .unwrap()
        .organism_id = OrganismId(2);
    assert_eq!(selected_readout(&world, stable).values, [None; 6]);
    world
        .get_mut::<ProductionCreatureAssemblyRoot>(entity)
        .unwrap()
        .organism_id = row.organism_id;
    assert_eq!(selected_readout(&world, stable), measured);
    world.remove_resource::<LiveBrainPresentationFrameResource>();
    assert_eq!(selected_readout(&world, stable).values, [None; 6]);
    world.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&organism_world).unwrap(),
    );
    assert_eq!(selected_readout(&world, stable), measured);
    let loaded = authoritative_world_with_energy(Some(0.15));
    let expected_reserve = loaded.presentation_snapshot().organisms[0]
        .biochemistry
        .body
        .energy;
    world
        .resource_mut::<LiveBrainPresentationFrameResource>()
        .reseed_from_loaded_world(&loaded)
        .unwrap();
    assert_eq!(
        selected_readout(&world, stable).values[1],
        Some(expected_reserve),
        "load rebinds to the new authoritative reserve immediately"
    );
    assert_ne!(selected_readout(&world, stable), measured);
    world
        .resource_mut::<LiveBrainPresentationFrameResource>()
        .reseed_from_loaded_world(&organism_world)
        .unwrap();
    assert_eq!(selected_readout(&world, stable), measured);
    world
        .entity_mut(entity)
        .remove::<Fvr04ProductionCreatureVisualMarker>();
    assert_eq!(selected_readout(&world, stable).values, [None; 6]);
}
#[test]
fn embedded_icons_decode_as_distinct_nonempty_rgba_images() {
    let mut hashes = BTreeSet::new();
    for (_, bytes) in EMBEDDED {
        let icon = image::load_from_memory(bytes).unwrap().to_rgba8();
        assert_eq!(icon.dimensions(), (96, 96));
        assert!(icon.pixels().any(|p| p[3] > 0));
        assert!(icon.pixels().any(|p| p[3] == 0));
        assert!(hashes.insert(blake3::hash(bytes).to_hex().to_string()));
    }
}
