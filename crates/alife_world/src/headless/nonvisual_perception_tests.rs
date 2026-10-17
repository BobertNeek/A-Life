use super::*;
use std::cell::Cell;

const OBSERVER: OrganismId = OrganismId(1);

thread_local! {
    static OBJECT_EVALUATIONS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record_object_evaluation() {
    OBJECT_EVALUATIONS.with(|count| count.set(count.get() + 1));
}

fn reset_evaluations() {
    OBJECT_EVALUATIONS.with(|count| count.set(0));
}

fn evaluations() -> usize {
    OBJECT_EVALUATIONS.with(Cell::get)
}

fn sparse_world(position: Vec3f, remote_count: usize) -> HeadlessWorld {
    let mut builder = HeadlessScenarioBuilder::new(71_016).agent("observer", OBSERVER, position);
    for index in 0..remote_count {
        builder = builder.food(
            &format!("remote-{index}"),
            Vec3f::new(1_024.0 + index as f32 * 16.0, 0.0, 1_024.0),
            0.1,
        );
    }
    builder.build().unwrap()
}

fn insert_food(
    world: &mut HeadlessWorld,
    label: &str,
    position: Vec3f,
    chemical: [f32; 2],
) -> WorldEntityId {
    let id = world
        .insert_object(SpawnSpec {
            label,
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition: 0.1,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: 0.0,
            teacher_channel: None,
        })
        .unwrap();
    world
        .objects
        .get_mut(&id.raw())
        .unwrap()
        .grounded_physical
        .chemical = [chemical[0], chemical[1], 0.0];
    id
}

fn assert_packet_matches(world: &HeadlessWorld) -> (PerceptionFrameDraft, usize, usize) {
    let index = world.build_perception_batch_index().unwrap();
    let mut canonical_world = world.clone();
    let mut indexed_world = world.clone();
    let tick = world.tick();
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    reset_evaluations();
    let canonical = canonical_world
        .perception_frame_draft(
            OBSERVER,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
        )
        .unwrap();
    let canonical_evaluations = evaluations();
    reset_evaluations();
    let indexed = indexed_world
        .perception_frame_draft_indexed(
            OBSERVER,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
            &index,
        )
        .unwrap();
    let indexed_evaluations = evaluations();
    assert_eq!(
        indexed.sensory().channels.as_flat_array().map(f32::to_bits),
        canonical
            .sensory()
            .channels
            .as_flat_array()
            .map(f32::to_bits),
        "all sensory float bits must match",
    );
    assert_eq!(indexed.candidates(), canonical.candidates());
    assert_eq!(
        indexed.grounded_object_slots(),
        canonical.grounded_object_slots()
    );
    assert_eq!(indexed, canonical, "complete perception draft must match");
    assert_eq!(
        serde_json::to_vec(&indexed).unwrap(),
        serde_json::to_vec(&canonical).unwrap()
    );
    (indexed, canonical_evaluations, indexed_evaluations)
}

#[test]
fn indexed_nonvisual_lateral_odor_reaches_cell_two_on_both_sides() {
    for sign in [-1.0_f32, 1.0] {
        let mut world = sparse_world(Vec3f::new(sign * 7.9, 0.0, 0.0), 128);
        let observer_id = world.agent_entity_id(OBSERVER).unwrap();
        world.objects.get_mut(&observer_id.raw()).unwrap().head_yaw = std::f32::consts::FRAC_PI_2;
        let source = insert_food(
            &mut world,
            "edge-odor",
            Vec3f::new(sign * 16.1, 0.0, 0.0),
            [0.3, -0.2],
        );
        let index = world.build_perception_batch_index().unwrap();
        let observer = world.agent_for(OBSERVER).unwrap();
        assert!(!world
            .indexed_nearby_objects(observer, &index)
            .unwrap()
            .iter()
            .any(|object| object.id == source));
        assert!(world
            .indexed_nonvisual_objects(observer, &index)
            .unwrap()
            .unwrap()
            .iter()
            .any(|object| object.id == source));
        let (draft, canonical, indexed) = assert_packet_matches(&world);
        assert!(draft
            .sensory()
            .channels
            .smell_chemistry
            .iter()
            .any(|value| *value > 0.0));
        assert_eq!(canonical, 129);
        assert_eq!(indexed, 1);
    }
}

#[test]
fn indexed_nonvisual_rear_occluded_odor_and_touch_keep_hidden_targets_hidden() {
    let mut world = sparse_world(Vec3f::ZERO, 128);
    insert_food(
        &mut world,
        "rear-odor",
        Vec3f::new(-2.0, 0.0, 2.0),
        [0.2, -0.1],
    );
    let contact = insert_food(
        &mut world,
        "rear-contact",
        Vec3f::new(-0.5, 0.0, -0.2),
        [0.0; 2],
    );
    world
        .objects
        .get_mut(&contact.raw())
        .unwrap()
        .grounded_physical
        .surface_temperature = -0.7;
    let (draft, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 2);
    assert!(draft.grounded_object_slots().is_empty());
    assert!(draft
        .candidates()
        .iter()
        .all(|candidate| candidate.target.entity.is_none()));
    assert_eq!(draft.sensory().channels.tactile_contact[0], 1.0);
    assert_eq!(draft.sensory().channels.tactile_contact[4], 0.7);
    assert!(draft.sensory().channels.smell_chemistry[0] > 0.0);
}

#[test]
fn indexed_nonvisual_far_oversized_contact_and_temperature_are_retained() {
    let mut world = sparse_world(Vec3f::ZERO, 256);
    let far = insert_food(
        &mut world,
        "large-contact",
        Vec3f::new(16.0, 0.0, 0.0),
        [0.0; 2],
    );
    let object = world.objects.get_mut(&far.raw()).unwrap();
    object.radius = 16.0;
    object.grounded_physical.surface_temperature = 0.8;
    let index = world.build_perception_batch_index().unwrap();
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_some());
    let (draft, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 1);
    assert_eq!(draft.sensory().channels.tactile_contact[0], 1.0);
    assert_eq!(draft.sensory().channels.tactile_contact[3], 0.8);
}

#[test]
fn indexed_nonvisual_far_possession_and_duplicate_membership_are_retained_once() {
    let mut world = sparse_world(Vec3f::ZERO, 128);
    let far = insert_food(
        &mut world,
        "far-carried",
        Vec3f::new(10_000.0, 0.0, 0.0),
        [0.1, -0.1],
    );
    let local = insert_food(
        &mut world,
        "local-carried",
        Vec3f::new(0.4, 0.0, 0.0),
        [0.1, -0.1],
    );
    for id in [far, local] {
        world.objects.get_mut(&id.raw()).unwrap().carried_by = Some(OBSERVER);
    }
    let index = world.build_perception_batch_index().unwrap();
    let candidates = world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .unwrap();
    assert_eq!(
        candidates
            .iter()
            .filter(|object| object.id == local)
            .count(),
        1
    );
    assert_eq!(
        candidates.iter().filter(|object| object.id == far).count(),
        1
    );
    let (draft, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 2);
    assert_eq!(draft.sensory().channels.tactile_contact[2], 1.0);
}

#[test]
fn indexed_nonvisual_self_and_consumed_fields_preserve_sparse_query() {
    let mut world = sparse_world(Vec3f::ZERO, 128);
    let self_id = world.agent_entity_id(OBSERVER).unwrap();
    world
        .objects
        .get_mut(&self_id.raw())
        .unwrap()
        .grounded_physical
        .chemical = [100.0; 3];
    let consumed = insert_food(
        &mut world,
        "consumed",
        Vec3f::new(0.2, 0.0, 0.0),
        [f32::NAN, f32::INFINITY],
    );
    let object = world.objects.get_mut(&consumed.raw()).unwrap();
    object.consumed = true;
    object.radius = f32::INFINITY;
    object.carried_by = Some(OBSERVER);
    let index = world.build_perception_batch_index().unwrap();
    assert!(!index.nonvisual_requires_full_scan);
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_some());
    let (draft, canonical, indexed) = assert_packet_matches(&world);
    assert_eq!(canonical, 128);
    assert_eq!(indexed, 0);
    assert_eq!(
        draft.sensory().channels.smell_chemistry,
        [0.0; SENSORY_SMELL_CHANNEL_COUNT]
    );
    assert_eq!(draft.sensory().channels.tactile_contact[2], 0.0);
}

#[test]
fn indexed_nonvisual_accumulates_four_mixed_sign_bands_in_entity_order() {
    // Keep this an indexed-order test below the conservative occupancy budget.
    let mut world = sparse_world(Vec3f::ZERO, 2_048);
    for number in 0..64 {
        let x = if number % 2 == 0 { -6.5 } else { 6.5 };
        let z = (number % 7) as f32 * 0.13 - 0.4;
        let magnitude = if number % 3 == 0 { 0.1 } else { 0.000_000_01 };
        let sign = if number % 4 < 2 { 1.0 } else { -1.0 };
        insert_food(
            &mut world,
            &format!("ordered-{number}"),
            Vec3f::new(x, 0.0, z),
            [magnitude * sign, -magnitude * sign],
        );
    }
    let index = world.build_perception_batch_index().unwrap();
    let candidates = world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .unwrap();
    let ids = candidates
        .iter()
        .map(|object| object.id.raw())
        .collect::<Vec<_>>();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let (draft, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 64);
    assert!(draft
        .sensory()
        .channels
        .smell_chemistry
        .iter()
        .all(|value| *value > 0.0 && *value < 1.0));
}

#[test]
fn indexed_nonvisual_rebuilt_batch_tracks_position_radius_and_owner_mutations() {
    let mut world = sparse_world(Vec3f::ZERO, 256);
    let target = insert_food(
        &mut world,
        "moved",
        Vec3f::new(100.0, 0.0, 0.0),
        [0.2, -0.3],
    );
    let (before, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 0);
    world
        .editor_move_object(target, Vec3f::new(-0.4, 0.0, 0.0))
        .unwrap();
    let (moved, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 1);
    assert_ne!(
        before.sensory().channels.smell_chemistry,
        moved.sensory().channels.smell_chemistry
    );
    let target = world.objects.get_mut(&target.raw()).unwrap();
    target.position = Vec3f::new(10_000.0, 0.0, 0.0);
    target.carried_by = Some(OBSERVER);
    let (owned, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 1);
    assert_eq!(owned.sensory().channels.tactile_contact[2], 1.0);
    let target = world.entity_id("moved").unwrap();
    let object = world.objects.get_mut(&target.raw()).unwrap();
    object.position = Vec3f::new(16.0, 0.0, 0.0);
    object.radius = 16.0;
    object.carried_by = None;
    let (resized, _, indexed) = assert_packet_matches(&world);
    assert_eq!(indexed, 1);
    assert_eq!(resized.sensory().channels.tactile_contact[0], 1.0);
    assert_eq!(resized.sensory().channels.tactile_contact[2], 0.0);
}

#[test]
fn indexed_nonvisual_huge_radius_and_small_world_choose_bounded_full_scan() {
    let mut small = sparse_world(Vec3f::ZERO, 0);
    insert_food(
        &mut small,
        "small-contact",
        Vec3f::new(0.5, 0.0, 0.0),
        [0.1, 0.0],
    );
    let (_, canonical, indexed) = assert_packet_matches(&small);
    assert_eq!(indexed, canonical);
    let mut huge = sparse_world(Vec3f::ZERO, 128);
    let id = insert_food(&mut huge, "huge", Vec3f::new(1.0e18, 0.0, 0.0), [0.0; 2]);
    huge.objects.get_mut(&id.raw()).unwrap().radius = 1.0e20;
    let index = huge.build_perception_batch_index().unwrap();
    assert!(huge
        .indexed_nonvisual_objects(huge.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_none());
    let (_, canonical, indexed) = assert_packet_matches(&huge);
    assert_eq!(indexed, canonical);
}

#[test]
fn indexed_nonvisual_nonfinite_edges_preserve_canonical_error_or_float_bits() {
    for chemical in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut world = sparse_world(Vec3f::ZERO, 128);
        let id = world.entity_id("remote-0").unwrap();
        world
            .objects
            .get_mut(&id.raw())
            .unwrap()
            .grounded_physical
            .chemical[0] = chemical;
        let index = world.build_perception_batch_index().unwrap();
        assert!(index.nonvisual_requires_full_scan);
        assert!(world
            .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
            .unwrap()
            .is_none());
        let mut canonical = world.clone();
        let mut indexed = world.clone();
        let arguments = (
            OBSERVER,
            Tick::ZERO,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(Tick::ZERO),
        );
        assert_eq!(
            canonical
                .perception_frame_draft(arguments.0, arguments.1, arguments.2, arguments.3)
                .unwrap_err(),
            indexed
                .perception_frame_draft_indexed(
                    arguments.0,
                    arguments.1,
                    arguments.2,
                    arguments.3,
                    &index
                )
                .unwrap_err(),
        );
    }
    for radius in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut world = sparse_world(Vec3f::ZERO, 128);
        let id = world.entity_id("remote-0").unwrap();
        world.objects.get_mut(&id.raw()).unwrap().radius = radius;
        let index = world.build_perception_batch_index().unwrap();
        assert!(index.nonvisual_requires_full_scan);
        assert_packet_matches(&world);
    }
    let mut world = sparse_world(Vec3f::ZERO, 128);
    let id = world.agent_entity_id(OBSERVER).unwrap();
    world.objects.get_mut(&id.raw()).unwrap().head_yaw = f32::NAN;
    let index = world.build_perception_batch_index().unwrap();
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_none());
    world.objects.get_mut(&id.raw()).unwrap().position.x = f32::INFINITY;
    assert!(world.build_perception_batch_index().is_err());
}

#[test]
fn indexed_nonvisual_cell_bounds_cover_boundary_float_neighbors_and_extreme_positions() {
    for coordinate in [
        -16.0_f32,
        -8.25,
        -8.0,
        -0.0,
        0.0,
        7.9,
        8.0,
        8.25,
        16.0,
        1.0e10,
        f32::MAX,
    ] {
        for x in [coordinate.next_down(), coordinate, coordinate.next_up()] {
            if !x.is_finite() {
                continue;
            }
            let mut world = sparse_world(Vec3f::new(x, 0.0, 0.0), 128);
            for (number, offset) in [-8.25_f32, -8.0, -0.75, 0.0, 0.75, 8.0, 8.25]
                .into_iter()
                .enumerate()
            {
                let position = Vec3f::new(x + offset, 0.0, 0.0);
                if position.x.is_finite() {
                    insert_food(
                        &mut world,
                        &format!("boundary-{number}"),
                        position,
                        [0.01, -0.02],
                    );
                }
            }
            assert_packet_matches(&world);
        }
    }
}

#[test]
fn indexed_nonvisual_work_counts_stay_local_when_remote_world_grows() {
    for remote_count in [128, 512, 2_048] {
        let mut world = sparse_world(Vec3f::ZERO, remote_count);
        insert_food(&mut world, "near", Vec3f::new(-0.5, 0.0, 0.0), [0.1, -0.1]);
        let far = insert_food(
            &mut world,
            "far-owned",
            Vec3f::new(10_000.0, 0.0, 0.0),
            [0.1, -0.1],
        );
        world.objects.get_mut(&far.raw()).unwrap().carried_by = Some(OBSERVER);
        let (_, canonical, indexed) = assert_packet_matches(&world);
        assert_eq!(canonical, remote_count + 2);
        assert_eq!(indexed, 2);
        println!("remote={remote_count} canonical_objects={canonical} indexed_objects={indexed} canonical_distances={} indexed_distances={}", 3 * canonical, 3 * indexed);
    }
}

#[test]
fn indexed_nonvisual_small_world_skips_only_unused_metadata_until_seventeen_objects() {
    let mut world = sparse_world(Vec3f::ZERO, 14);
    let carried = insert_food(
        &mut world,
        "small-carried",
        Vec3f::new(10_000.0, 0.0, 0.0),
        [0.1, -0.1],
    );
    let object = world.objects.get_mut(&carried.raw()).unwrap();
    object.carried_by = Some(OBSERVER);
    object.radius = 9.0;
    let index = world.build_perception_batch_index().unwrap();
    assert_eq!(index.object_count, 16);
    assert!(index.nonvisual_requires_full_scan);
    assert_eq!(index.max_contact_radius, HEADLESS_CONTACT_RADIUS);
    assert!(index.carried_entities.is_empty());
    assert_eq!(index.cells.values().map(Vec::len).sum::<usize>(), 16);
    assert_eq!(index.organism_entities.len(), 1);
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_none());
    let (draft, canonical, indexed) = assert_packet_matches(&world);
    assert_eq!(canonical, indexed);
    assert_eq!(draft.sensory().channels.tactile_contact[2], 1.0);

    insert_food(
        &mut world,
        "seventeenth",
        Vec3f::new(20_000.0, 0.0, 0.0),
        [0.0; 2],
    );
    let index = world.build_perception_batch_index().unwrap();
    assert_eq!(index.object_count, 17);
    assert!(!index.nonvisual_requires_full_scan);
    assert_eq!(index.max_contact_radius, 9.0);
    assert_eq!(index.carried_entities[&OBSERVER.raw()], vec![carried.raw()]);
    assert_packet_matches(&world);
}

#[test]
fn indexed_nonvisual_small_world_keeps_existing_index_validation() {
    let mut invalid_position = sparse_world(Vec3f::ZERO, 0);
    let observer = invalid_position.agent_entity_id(OBSERVER).unwrap();
    invalid_position
        .objects
        .get_mut(&observer.raw())
        .unwrap()
        .position
        .x = f32::INFINITY;
    assert!(invalid_position.build_perception_batch_index().is_err());

    let mut duplicate_organism = sparse_world(Vec3f::ZERO, 1);
    let remote = duplicate_organism.entity_id("remote-0").unwrap();
    duplicate_organism
        .objects
        .get_mut(&remote.raw())
        .unwrap()
        .organism_id = Some(OBSERVER);
    assert!(duplicate_organism.build_perception_batch_index().is_err());
}

#[test]
fn indexed_nonvisual_dense_cell_falls_back_with_exact_mixed_band_packet() {
    let mut world = sparse_world(Vec3f::ZERO, 0);
    for number in 0..127 {
        let magnitude = if number % 3 == 0 { 0.01 } else { 0.000_000_01 };
        let sign = if number % 4 < 2 { 1.0 } else { -1.0 };
        let id = insert_food(
            &mut world,
            &format!("dense-{number}"),
            Vec3f::new(0.2, 0.0, 0.1),
            [magnitude * sign, -magnitude * sign],
        );
        world
            .objects
            .get_mut(&id.raw())
            .unwrap()
            .grounded_physical
            .surface_temperature = if number % 2 == 0 { 0.7 } else { -0.8 };
    }
    let index = world.build_perception_batch_index().unwrap();
    assert_eq!(index.object_count, 128);
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
        .unwrap()
        .is_none());
    let (draft, canonical, indexed) = assert_packet_matches(&world);
    assert_eq!(canonical, 127);
    assert_eq!(indexed, canonical);
    assert_eq!(draft.sensory().channels.tactile_contact[3], 0.7);
    assert_eq!(draft.sensory().channels.tactile_contact[4], 0.8);
}

#[test]
fn indexed_nonvisual_cell_occupancy_falls_back_at_ceil_sixteenth() {
    // 257 total objects gives a raw candidate budget of ceil(257 / 16) = 17.
    // The observer also occupies a queried cell and conservatively counts.
    for split_cells in [false, true] {
        for near_count in [15, 16] {
            let mut world = sparse_world(Vec3f::ZERO, 256 - near_count);
            for number in 0..near_count {
                // With split cells, each is individually below budget and only
                // the cumulative raw-ID count can trigger the fallback.
                let x = if split_cells && number % 2 == 0 {
                    -0.2
                } else {
                    0.2
                };
                insert_food(
                    &mut world,
                    &format!("near-budget-{number}"),
                    Vec3f::new(x, 0.0, 0.1),
                    [0.01, -0.02],
                );
            }
            let index = world.build_perception_batch_index().unwrap();
            assert_eq!(index.object_count, 257);
            let candidates = world
                .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
                .unwrap();
            if near_count == 15 {
                assert_eq!(candidates.unwrap().len(), 16);
            } else {
                assert!(candidates.is_none());
            }
            let (_, canonical, indexed) = assert_packet_matches(&world);
            assert_eq!(canonical, 256);
            assert_eq!(indexed, if near_count == 15 { 15 } else { canonical });
        }
    }
}

#[test]
fn indexed_nonvisual_far_carried_union_obeys_the_same_raw_budget() {
    for carried_count in [15, 16] {
        let mut world = sparse_world(Vec3f::ZERO, 256);
        for number in 0..carried_count {
            let id = world.entity_id(&format!("remote-{number}")).unwrap();
            world.objects.get_mut(&id.raw()).unwrap().carried_by = Some(OBSERVER);
        }
        let index = world.build_perception_batch_index().unwrap();
        let candidates = world
            .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
            .unwrap();
        if carried_count == 15 {
            assert_eq!(candidates.unwrap().len(), 16);
        } else {
            assert!(candidates.is_none());
        }
        let (draft, canonical, indexed) = assert_packet_matches(&world);
        assert_eq!(indexed, if carried_count == 15 { 15 } else { canonical });
        assert_eq!(draft.sensory().channels.tactile_contact[2], 1.0);
    }
}

#[test]
fn indexed_nonvisual_duplicate_membership_can_only_choose_an_earlier_full_scan() {
    for local_carried_count in [7, 8] {
        let mut world = sparse_world(Vec3f::ZERO, 256 - local_carried_count);
        for number in 0..local_carried_count {
            let id = insert_food(
                &mut world,
                &format!("local-carried-budget-{number}"),
                Vec3f::new(0.2, 0.0, 0.1),
                [0.01, -0.02],
            );
            world.objects.get_mut(&id.raw()).unwrap().carried_by = Some(OBSERVER);
        }
        let index = world.build_perception_batch_index().unwrap();
        let candidates = world
            .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
            .unwrap();
        if local_carried_count == 7 {
            // Seven local ownership duplicates were deduplicated, in ID order.
            assert_eq!(candidates.unwrap().len(), 8);
        } else {
            // 9 spatial + 8 ownership IDs hit the raw budget before dedup.
            assert!(candidates.is_none());
        }
        let (_, canonical, indexed) = assert_packet_matches(&world);
        assert_eq!(
            indexed,
            if local_carried_count == 7 {
                7
            } else {
                canonical
            }
        );
    }
}

// Append-only tests for the rebuilt candidate's existing nonvisual unit module.
// These exercise a reused index, unlike assert_packet_matches, which rebuilds it.
// No production behavior or index invalidation contract is changed here.

fn live_read_register_observer(world: &mut HeadlessWorld) {
    let genome = alife_core::CreatureGenome::early_mammal_founder(
        0xE107_1017,
        alife_core::FoundationGeneticIdentity::new(
            10,
            1,
            7,
            alife_core::BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biology = BiochemistryState::new(&phenotype, world.tick()).unwrap();
    let record = WorldOrganismRecord::new(
        OBSERVER,
        world.agent_entity_id(OBSERVER).unwrap(),
        genome,
        phenotype,
        biology,
        world.tick(),
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    live_read_set_sensor_calibration(world, 0.0);
}

fn live_read_set_sensor_calibration(world: &mut HeadlessWorld, value: f32) {
    let mut record = world.organism_registry().get(OBSERVER).unwrap().clone();
    let mut embodiment = record.embodiment().clone();
    let sensors = vec![value; embodiment.sensor_calibration().len()];
    let effectors = embodiment.effector_controllability().to_vec();
    let body_schema = embodiment.body_schema().to_vec();
    embodiment
        .replace_calibration(world.tick(), sensors, effectors, body_schema)
        .unwrap();
    record.replace_embodiment_state(embodiment).unwrap();
    world.replace_organism_record_exact(record).unwrap();
}

fn live_read_world() -> (HeadlessWorld, WorldEntityId) {
    let mut world = sparse_world(Vec3f::ZERO, 256);
    let source = insert_food(
        &mut world,
        "live-source",
        Vec3f::new(2.0, 0.0, 0.0),
        [0.12, -0.18],
    );
    live_read_register_observer(&mut world);
    (world, source)
}

fn live_read_capture_pair(
    world: &HeadlessWorld,
    index: &HeadlessPerceptionBatchIndex,
) -> (
    Result<PerceptionFrameDraft, ScaffoldContractError>,
    Result<PerceptionFrameDraft, ScaffoldContractError>,
) {
    // Both captures start with identical tracking state. The supplied index is
    // deliberately not rebuilt after the caller's mutation.
    let mut canonical_world = world.clone();
    let mut indexed_world = world.clone();
    let tick = world.tick();
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    (
        canonical_world.perception_frame_draft(
            OBSERVER,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
        ),
        indexed_world.perception_frame_draft_indexed(
            OBSERVER,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
            index,
        ),
    )
}

fn live_read_assert_packet_matches(
    world: &HeadlessWorld,
    index: &HeadlessPerceptionBatchIndex,
) -> PerceptionFrameDraft {
    let (canonical, indexed) = live_read_capture_pair(world, index);
    let canonical = canonical.unwrap();
    let indexed = indexed.unwrap();
    assert_eq!(
        indexed.sensory().channels.as_flat_array().map(f32::to_bits),
        canonical
            .sensory()
            .channels
            .as_flat_array()
            .map(f32::to_bits),
    );
    assert_eq!(indexed, canonical, "complete draft with reused index");
    assert_eq!(
        serde_json::to_vec(&indexed).unwrap(),
        serde_json::to_vec(&canonical).unwrap(),
    );
    indexed
}

fn live_read_assert_sparse(world: &HeadlessWorld, index: &HeadlessPerceptionBatchIndex) {
    assert!(index.object_count > NONVISUAL_SMALL_WORLD_OBJECTS);
    assert!(!index.nonvisual_requires_full_scan);
    assert!(world
        .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), index)
        .unwrap()
        .is_some());
}

#[test]
fn indexed_nonvisual_reused_index_reads_current_observer_sensor_gains() {
    let (mut world, _) = live_read_world();
    world
        .organism_registry
        .with_biology_mut(OBSERVER, |biology| {
            // Injury is a projection of organ damage; preserve that invariant.
            biology.body.set_health(0.625)?;
            Ok(())
        })
        .unwrap();
    world
        .queue_player_care(OBSERVER, Vec3f::ZERO, false)
        .unwrap();
    world.ingestion_observations.insert(
        OBSERVER.raw(),
        IngestionObservation {
            ingested_at: world.tick(),
            taste: 0.25,
        },
    );
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    let source_objects_before = world.object_snapshots();

    // Supported authoritative registry replacement, after index construction.
    live_read_set_sensor_calibration(&mut world, -1.0);
    assert_eq!(world.object_snapshots(), source_objects_before);
    live_read_assert_sparse(&world, &index);
    let after = live_read_assert_packet_matches(&world, &index);
    for (old, new) in before
        .sensory()
        .channels
        .smell_chemistry
        .iter()
        .zip(after.sensory().channels.smell_chemistry.iter())
    {
        assert_eq!(*new, *old * 0.5);
    }
    assert!(before.sensory().channels.smell_chemistry[0] > 0.0);
    assert_eq!(before.sensory().channels.pain_signal.raw(), 0.375);
    assert_eq!(after.sensory().channels.pain_signal.raw(), 0.1875);
    assert_eq!(before.sensory().channels.tactile_contact[0], 0.25);
    assert_eq!(after.sensory().channels.tactile_contact[0], 0.125);
    assert_eq!(before.sensory().channels.tactile_contact[5], 0.25);
    // Existing canonical semantics apply Chemical and then Touch gain to taste.
    assert_eq!(after.sensory().channels.tactile_contact[5], 0.0625);
}

#[test]
fn indexed_nonvisual_reused_index_reads_current_registry_injury() {
    let (mut world, _) = live_read_world();
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    let source_objects_before = world.object_snapshots();
    world
        .organism_registry
        .with_biology_mut(OBSERVER, |biology| {
            biology.body.set_health(0.375)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(world.object_snapshots(), source_objects_before);
    let after = live_read_assert_packet_matches(&world, &index);
    assert_ne!(before.sensory().channels.pain_signal.raw(), 0.625);
    assert_eq!(after.sensory().channels.pain_signal.raw(), 0.625);
}

#[test]
fn indexed_nonvisual_reused_index_reads_current_player_care() {
    let (mut world, _) = live_read_world();
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    assert_eq!(before.sensory().channels.tactile_contact[0], 0.0);
    assert_eq!(before.sensory().channels.auditory_acoustic[1], 0.0);
    let source_objects_before = world.object_snapshots();
    world
        .queue_player_care(OBSERVER, Vec3f::ZERO, false)
        .unwrap();
    world
        .queue_player_care(OBSERVER, Vec3f::ZERO, true)
        .unwrap();
    assert_eq!(world.object_snapshots(), source_objects_before);
    let after = live_read_assert_packet_matches(&world, &index);
    assert_eq!(after.sensory().channels.tactile_contact[0], 0.25);
    assert_eq!(after.sensory().channels.auditory_acoustic[1], 1.0);
}

#[test]
fn indexed_nonvisual_reused_index_reads_current_ingestion_sample_and_removal() {
    let (mut world, _) = live_read_world();
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let source_objects_before = world.object_snapshots();
    // Private fixture insertion isolates taste state from an Eat action's
    // separate source-object mutations; it is not an alternate ingestion API.
    for taste in [0.25_f32, -0.5] {
        world.ingestion_observations.insert(
            OBSERVER.raw(),
            IngestionObservation {
                ingested_at: world.tick(),
                taste,
            },
        );
        let draft = live_read_assert_packet_matches(&world, &index);
        assert_eq!(draft.sensory().channels.tactile_contact[5], taste.max(0.0));
        assert_eq!(
            draft.sensory().channels.tactile_contact[6],
            (-taste).max(0.0)
        );
    }
    world.ingestion_observations.remove(&OBSERVER.raw());
    let removed = live_read_assert_packet_matches(&world, &index);
    assert_eq!(removed.sensory().channels.tactile_contact[5], 0.0);
    assert_eq!(removed.sensory().channels.tactile_contact[6], 0.0);
    assert_eq!(world.object_snapshots(), source_objects_before);
}

#[test]
fn indexed_nonvisual_reused_index_observes_lifecycle_ingestion_pruning() {
    let (mut world, _) = live_read_world();
    world.ingestion_observations.insert(
        OBSERVER.raw(),
        IngestionObservation {
            ingested_at: world.tick(),
            taste: 0.5,
        },
    );
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    assert_eq!(before.sensory().channels.tactile_contact[5], 0.5);
    let source_objects_before = world.object_snapshots();
    world
        .organism_registry
        .mark_dead(OBSERVER, world.tick())
        .unwrap();
    // The sensor also independently filters a dead observer's sample.
    let filtered = live_read_assert_packet_matches(&world, &index);
    assert_eq!(filtered.sensory().channels.tactile_contact[5], 0.0);
    let dead_record = world.organism_registry().get(OBSERVER).unwrap().clone();
    // Use the same public replacement path that performs production pruning.
    world.replace_organism_record_exact(dead_record).unwrap();
    assert!(!world.ingestion_observations.contains_key(&OBSERVER.raw()));
    let pruned = live_read_assert_packet_matches(&world, &index);
    assert_eq!(pruned.sensory().channels.tactile_contact[5], 0.0);
    assert_eq!(world.object_snapshots(), source_objects_before);
}

#[test]
fn indexed_nonvisual_reused_index_reads_selected_source_payload_and_consumption() {
    let (mut world, source) = live_read_world();
    world
        .editor_move_object(source, Vec3f::new(-0.4, 0.0, 0.0))
        .unwrap();
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    let mut properties = world.entity(source).unwrap().grounded_physical;
    properties.chemical = [0.3, -0.05, 0.0];
    properties.surface_temperature = 0.625;
    world
        .set_grounded_physical_properties(source, properties)
        .unwrap();
    let changed = live_read_assert_packet_matches(&world, &index);
    assert_ne!(
        changed.sensory().channels.smell_chemistry,
        before.sensory().channels.smell_chemistry,
    );
    assert_eq!(changed.sensory().channels.tactile_contact[3], 0.625);
    // Removing this source's influence is safe for this existing superset.
    // This does not promise arbitrary revival/ownership/radius mutation support.
    world.objects.get_mut(&source.raw()).unwrap().consumed = true;
    let consumed = live_read_assert_packet_matches(&world, &index);
    assert_eq!(consumed.sensory().channels.smell_chemistry, [0.0; 8]);
    assert_eq!(consumed.sensory().channels.tactile_contact[3], 0.0);
}

#[test]
fn indexed_nonvisual_reused_index_reads_current_observer_pose_and_yaw() {
    // This checks only this observer's own query over fixed other sources.
    // Its moved body could invalidate source cells used by another observer.
    let mut world = sparse_world(Vec3f::ZERO, 256);
    insert_food(
        &mut world,
        "fixed-source",
        Vec3f::new(100.0, 0.0, 0.0),
        [0.2, -0.1],
    );
    let index = world.build_perception_batch_index().unwrap();
    live_read_assert_sparse(&world, &index);
    let before = live_read_assert_packet_matches(&world, &index);
    assert_eq!(before.sensory().channels.smell_chemistry, [0.0; 8]);
    let observer = world.agent_entity_id(OBSERVER).unwrap();
    world
        .editor_move_object(observer, Vec3f::new(99.0, 0.0, 0.0))
        .unwrap();
    world.objects.get_mut(&observer.raw()).unwrap().head_yaw = 1.0;
    live_read_assert_sparse(&world, &index);
    let after = live_read_assert_packet_matches(&world, &index);
    assert!(after.sensory().channels.smell_chemistry[0] > 0.0);
    assert_ne!(
        after.sensory().channels.smell_chemistry[0],
        after.sensory().channels.smell_chemistry[1],
    );
}

#[test]
fn indexed_nonvisual_unsupported_same_tick_source_reuse_requires_rebuild() {
    // Characterization of unsupported reuse, not a production staged-tick bug.
    // Every current production capture precedes these source-object changes.
    for mutation in ["position", "radius", "ownership"] {
        let mut world = sparse_world(Vec3f::ZERO, 256);
        let target = insert_food(
            &mut world,
            "mutated-source",
            Vec3f::new(100.0, 0.0, 0.0),
            [0.2, -0.1],
        );
        let index = world.build_perception_batch_index().unwrap();
        live_read_assert_sparse(&world, &index);
        live_read_assert_packet_matches(&world, &index);
        match mutation {
            "position" => world
                .editor_move_object(target, Vec3f::new(-0.4, 0.0, 0.0))
                .unwrap(),
            "radius" => world.objects.get_mut(&target.raw()).unwrap().radius = 100.0,
            "ownership" => {
                world.objects.get_mut(&target.raw()).unwrap().carried_by = Some(OBSERVER);
            }
            _ => unreachable!(),
        }
        // Seed/tick/count are deliberately unchanged and the guard accepts it.
        world.validate_perception_batch_index(&index).unwrap();
        live_read_assert_sparse(&world, &index);
        let (canonical, stale) = live_read_capture_pair(&world, &index);
        assert_ne!(
            canonical
                .unwrap()
                .sensory()
                .channels
                .as_flat_array()
                .map(f32::to_bits),
            stale
                .unwrap()
                .sensory()
                .channels
                .as_flat_array()
                .map(f32::to_bits),
            "unsupported stale source metadata: {mutation}",
        );
        let rebuilt = world.build_perception_batch_index().unwrap();
        live_read_assert_packet_matches(&world, &rebuilt);
    }
}

#[test]
fn indexed_nonvisual_unsupported_nonfinite_transition_requires_rebuild() {
    // Deliberately invalid private fixture: public physical-property validation
    // rejects nonfinite chemistry. This probes the fallback's reuse boundary.
    for chemical in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut world = sparse_world(Vec3f::ZERO, 256);
        let target = world.entity_id("remote-0").unwrap();
        let index = world.build_perception_batch_index().unwrap();
        live_read_assert_sparse(&world, &index);
        world
            .objects
            .get_mut(&target.raw())
            .unwrap()
            .grounded_physical
            .chemical[0] = chemical;
        world.validate_perception_batch_index(&index).unwrap();
        let (canonical, stale) = live_read_capture_pair(&world, &index);
        let canonical_error = canonical.unwrap_err();
        assert!(
            stale.is_ok(),
            "old metadata cannot see a remote invalid transition"
        );
        let rebuilt = world.build_perception_batch_index().unwrap();
        assert!(rebuilt.nonvisual_requires_full_scan);
        let (canonical, indexed) = live_read_capture_pair(&world, &rebuilt);
        assert_eq!(canonical.unwrap_err(), canonical_error);
        assert_eq!(indexed.unwrap_err(), canonical_error);
    }
}

#[test]
fn indexed_nonvisual_own_cell_is_inside_successful_bounds_at_geometry_edges() {
    // The early occupancy shortcut relies on this containment, including
    // subnormal rounding and saturating float-to-cell conversions.
    for coordinate in [
        -16.0_f32,
        -8.0,
        -0.0,
        0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        8.0,
        16.0,
        -1.0e10,
        1.0e10,
        -f32::MAX,
        f32::MAX,
    ] {
        for coordinate in [coordinate.next_down(), coordinate, coordinate.next_up()] {
            if !coordinate.is_finite() {
                continue;
            }
            for position in [
                Vec3f::new(coordinate, 0.0, 0.0),
                Vec3f::new(0.0, coordinate, 0.0),
                Vec3f::new(0.0, 0.0, coordinate),
            ] {
                for yaw in [0.0_f32, -2.7, 1.0e20, f32::MAX, f32::NAN] {
                    let lateral = Vec3f::new(-yaw.sin() * 0.25, 0.0, yaw.cos() * 0.25);
                    let samples = [add(position, lateral), subtract(position, lateral)];
                    for radius in [HEADLESS_CONTACT_RADIUS, 8.0, 1_024.0, f32::MAX] {
                        if let Some(bounds) = nonvisual_cell_bounds(position, samples, radius) {
                            let cell = perception_cell(position);
                            assert!((bounds.minimum.0..=bounds.maximum.0).contains(&cell.0));
                            assert!((bounds.minimum.1..=bounds.maximum.1).contains(&cell.1));
                            assert!((bounds.minimum.2..=bounds.maximum.2).contains(&cell.2));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn indexed_nonvisual_own_cell_raw_budget_keeps_consumed_members_and_packet_parity() {
    for object_count in [128_usize, 129] {
        let limit = object_count.div_ceil(NONVISUAL_CANDIDATE_DENSITY_DIVISOR);
        let mut world = sparse_world(Vec3f::ZERO, object_count - limit);
        let mut last = None;
        for number in 1..limit {
            last = Some(insert_food(
                &mut world,
                &format!("own-budget-{number}"),
                Vec3f::new(0.2, 0.0, 0.1),
                [0.01, -0.02],
            ));
        }
        let last = last.unwrap();
        let consumed = world.objects.get_mut(&last.raw()).unwrap();
        consumed.consumed = true;
        consumed.carried_by = Some(OBSERVER);
        let index = world.build_perception_batch_index().unwrap();
        let observer = world.agent_for(OBSERVER).unwrap();
        assert_eq!(index.object_count, object_count);
        assert!(!index.nonvisual_requires_full_scan);
        assert_eq!(
            index.cells[&perception_cell(observer.position)].len(),
            limit
        );
        assert!(world
            .indexed_nonvisual_objects(observer, &index)
            .unwrap()
            .is_none());
        let (_, canonical, indexed) = assert_packet_matches(&world);
        assert_eq!(indexed, canonical);

        // Rebuild after moving the consumed member: below the same raw budget,
        // the sparse path remains available and returns canonical ID order.
        world.objects.get_mut(&last.raw()).unwrap().position = Vec3f::new(20_000.0, 0.0, 0.0);
        let index = world.build_perception_batch_index().unwrap();
        let candidates = world
            .indexed_nonvisual_objects(world.agent_for(OBSERVER).unwrap(), &index)
            .unwrap()
            .unwrap();
        assert_eq!(candidates.len(), limit - 1);
        assert!(candidates
            .windows(2)
            .all(|pair| pair[0].id.raw() < pair[1].id.raw()));
        let (_, canonical, indexed) = assert_packet_matches(&world);
        assert!(indexed < canonical);
        assert_eq!(indexed, limit - 2);
    }
}

#[test]
fn indexed_nonvisual_dense_own_cell_never_bypasses_snapshot_validation() {
    let mut world = sparse_world(Vec3f::ZERO, 31);
    for object in world.objects.values_mut() {
        object.position = Vec3f::ZERO;
    }
    let index = world.build_perception_batch_index().unwrap();
    let observer = world.agent_for(OBSERVER).unwrap();
    assert!(world
        .indexed_nonvisual_objects(observer, &index)
        .unwrap()
        .is_none());
    for mutation in 0..3 {
        let mut stale = index.clone();
        match mutation {
            0 => stale.world_seed ^= 1,
            1 => stale.world_tick = Tick(stale.world_tick.0 + 1),
            _ => stale.object_count += 1,
        }
        assert_eq!(
            world
                .indexed_nonvisual_objects(observer, &stale)
                .unwrap_err(),
            ScaffoldContractError::InvalidPerceptionFrame,
        );
    }
}
