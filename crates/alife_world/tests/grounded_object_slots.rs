use alife_core::{
    BrainScaleTier, CandidateActionFamily, HomeostaticSnapshot, OrganismId, SensorProfile, Tick,
    TrackedObjectId, Vec3f,
};
use alife_world::{
    AssetManifest, GroundedPhysicalProperties, HeadlessActionIds, HeadlessScenarioBuilder,
    PhysicalTrackingProvenance, PortableSaveFile, RuntimeConfig, StablePhysicalDescriptor,
    TrackedObjectRegistry,
};

const ORGANISM: OrganismId = OrganismId(1);

fn cyan_bitter() -> GroundedPhysicalProperties {
    GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [0.0, 1.0, 1.0],
        material: [0.2, 0.8, 0.4],
        shape: [0.8, 0.1, 0.3],
        chemical: [-0.9, 0.1, 0.0],
        surface_temperature: -0.2,
        terrain: [0.6, 0.2],
    }
}

fn amber_sweet() -> GroundedPhysicalProperties {
    GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [1.0, 0.55, 0.1],
        material: [0.7, 0.3, 0.2],
        shape: [0.4, 0.8, 0.2],
        chemical: [0.4, 0.0, 0.0],
        surface_temperature: 0.1,
        terrain: [0.6, 0.2],
    }
}

fn two_physical_object_fixture() -> (
    alife_world::HeadlessWorld,
    alife_core::WorldEntityId,
    alife_core::WorldEntityId,
) {
    let world = HeadlessScenarioBuilder::new(4_303)
        .agent("agent", ORGANISM, Vec3f::ZERO)
        .food("cyan", Vec3f::new(1.0, 0.0, 0.0), 0.6)
        .grounded_physical("cyan", cyan_bitter())
        .hazard("amber", Vec3f::new(2.0, 0.0, 0.0), 0.7)
        .grounded_physical("amber", amber_sweet())
        .build()
        .unwrap();
    let cyan = world.entity_id("cyan").unwrap();
    let amber = world.entity_id("amber").unwrap();
    (world, cyan, amber)
}

fn grounded_draft(
    world: &mut alife_world::HeadlessWorld,
    tick: Tick,
) -> alife_core::PerceptionFrameDraft {
    world
        .perception_frame_draft(
            ORGANISM,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(tick),
        )
        .unwrap()
}

fn generic_families() -> [CandidateActionFamily; 5] {
    [
        CandidateActionFamily::Inspect,
        CandidateActionFamily::Approach,
        CandidateActionFamily::Avoid,
        CandidateActionFamily::Ingest,
        CandidateActionFamily::Contact,
    ]
}

fn action_ids_for_target(
    frame: &alife_core::PerceptionFrameDraft,
    target: alife_core::WorldEntityId,
) -> Vec<alife_core::ActionId> {
    frame
        .candidates()
        .iter()
        .filter(|candidate| candidate.target.entity == Some(target))
        .map(|candidate| candidate.action_id)
        .collect()
}

#[test]
fn grounded_profile_exposes_physics_not_world_object_kind() {
    let (mut world, cyan, amber) = two_physical_object_fixture();
    let frame = grounded_draft(&mut world, Tick::new(5));

    assert_eq!(frame.grounded_object_slots().len(), 2);
    assert!(frame
        .sensory()
        .channels
        .visual_affordance
        .iter()
        .all(|value| *value == 0.0));
    let expected = vec![
        alife_core::ActionKind::Inspect.canonical_id(),
        HeadlessActionIds::APPROACH,
        HeadlessActionIds::FLEE,
        HeadlessActionIds::EAT,
        HeadlessActionIds::GRAB,
    ];
    assert_eq!(action_ids_for_target(&frame, cyan), expected);
    assert_eq!(action_ids_for_target(&frame, amber), expected);
}

#[test]
fn tracked_ids_are_stable_while_candidate_features_remain_physical() {
    let (mut world, _, _) = two_physical_object_fixture();
    let first = grounded_draft(&mut world, Tick::new(5));
    let second = grounded_draft(&mut world, Tick::new(6));

    assert_eq!(
        first.grounded_object_slots()[0].tracked_object_id,
        second.grounded_object_slots()[0].tracked_object_id
    );
    assert_eq!(
        first.grounded_object_slots()[0]
            .candidate_features()
            .unwrap(),
        second.grounded_object_slots()[0]
            .candidate_features()
            .unwrap()
    );
}

#[test]
fn relabelling_private_world_semantics_cannot_change_a_grounded_frame() {
    let mut first = HeadlessScenarioBuilder::new(4_304)
        .agent("agent", ORGANISM, Vec3f::ZERO)
        .food("first", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .grounded_physical("first", cyan_bitter())
        .hazard("second", Vec3f::new(2.0, 0.0, 0.0), 0.8)
        .grounded_physical("second", amber_sweet())
        .build()
        .unwrap();
    let mut second = HeadlessScenarioBuilder::new(4_304)
        .agent("agent", ORGANISM, Vec3f::ZERO)
        .hazard("first", Vec3f::new(1.0, 0.0, 0.0), 0.2)
        .grounded_physical("first", cyan_bitter())
        .food("second", Vec3f::new(2.0, 0.0, 0.0), 0.1)
        .grounded_physical("second", amber_sweet())
        .build()
        .unwrap();

    let a = grounded_draft(&mut first, Tick::new(5));
    let b = grounded_draft(&mut second, Tick::new(5));
    assert_eq!(a, b);
    assert_eq!(a.sensory().channels.nearby_affordances.raw(), 0);
}

#[test]
fn sixteen_slots_yield_six_complete_family_groups_and_never_a_partial_group() {
    let mut builder = HeadlessScenarioBuilder::new(4_305).agent("agent", ORGANISM, Vec3f::ZERO);
    for index in 0..16 {
        let label = format!("object-{index}");
        let angle = index as f32 * std::f32::consts::TAU / 16.0;
        builder = builder
            .food(
                &label,
                Vec3f::new(angle.cos() * 4.0, angle.sin() * 4.0, 0.0),
                0.5,
            )
            .grounded_physical(&label, cyan_bitter());
    }
    let mut world = builder.build().unwrap();
    let frame = grounded_draft(&mut world, Tick::new(5));

    assert_eq!(frame.grounded_object_slots().len(), 16);
    assert_eq!(frame.candidates().len(), 1 + 6 * 5);
    assert_eq!(frame.candidates()[0].family, CandidateActionFamily::Idle);
    for group in frame.candidates()[1..].chunks_exact(5) {
        assert_eq!(
            group
                .iter()
                .map(|candidate| candidate.family)
                .collect::<Vec<_>>(),
            generic_families()
        );
        assert!(group
            .iter()
            .all(|candidate| candidate.observation == group[0].observation));
    }
}

#[test]
fn duplicate_looking_objects_keep_distinct_tracked_bindings() {
    let mut world = HeadlessScenarioBuilder::new(4_306)
        .agent("agent", ORGANISM, Vec3f::ZERO)
        .food("one", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .grounded_physical("one", cyan_bitter())
        .food("two", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .grounded_physical("two", cyan_bitter())
        .build()
        .unwrap();
    let frame = grounded_draft(&mut world, Tick::new(5));

    assert_eq!(
        frame.candidates()[1].features,
        frame.candidates()[6].features
    );
    assert_ne!(
        frame.candidates()[1].observation,
        frame.candidates()[6].observation
    );
    assert_ne!(
        frame.grounded_object_slots()[0].tracked_object_id,
        frame.grounded_object_slots()[1].tracked_object_id
    );
}

fn provenance(world_seed: u64, spawn_sequence: u64) -> PhysicalTrackingProvenance {
    PhysicalTrackingProvenance {
        schema_version: 1,
        world_seed,
        zone_id: 0,
        spawn_sequence,
        lineage_key: 0,
    }
}

fn descriptor() -> StablePhysicalDescriptor {
    StablePhysicalDescriptor::try_from(cyan_bitter()).unwrap()
}

#[test]
fn per_organism_tracked_ids_do_not_depend_on_cross_organism_schedule_order() {
    fn run(order: [OrganismId; 2]) -> [Vec<alife_world::TrackedObjectRecord>; 2] {
        let mut registry = TrackedObjectRegistry::new(99, 8).unwrap();
        for organism in order {
            registry
                .observe(organism, provenance(99, 1), descriptor(), Tick::new(4))
                .unwrap();
        }
        [
            registry
                .records_for(OrganismId(1))
                .unwrap()
                .copied()
                .collect(),
            registry
                .records_for(OrganismId(2))
                .unwrap()
                .copied()
                .collect(),
        ]
    }

    assert_eq!(
        run([OrganismId(1), OrganismId(2)]),
        run([OrganismId(2), OrganismId(1)])
    );
}

#[test]
fn tracker_capacity_evicts_deterministically_and_never_reuses_an_id() {
    fn run() -> (
        TrackedObjectRegistry,
        Vec<alife_world::TrackedObjectObservationReceipt>,
    ) {
        let mut registry = TrackedObjectRegistry::new(99, 2).unwrap();
        let mut receipts = Vec::new();
        for sequence in 1..=3 {
            receipts.push(
                registry
                    .observe(
                        ORGANISM,
                        provenance(99, sequence),
                        descriptor(),
                        Tick::new(sequence),
                    )
                    .unwrap(),
            );
        }
        (registry, receipts)
    }

    let (mut first, a) = run();
    let (_, b) = run();
    assert_eq!(a, b);
    assert_eq!(a[2].evicted, Some(a[0].tracked_object_id));
    assert_eq!(first.records_for(ORGANISM).unwrap().len(), 2);
    let reappeared = first
        .observe(ORGANISM, provenance(99, 1), descriptor(), Tick::new(12))
        .unwrap();
    assert!(reappeared.tracked_object_id.raw() > a[2].tracked_object_id.raw());
    assert_ne!(reappeared.tracked_object_id, a[0].tracked_object_id);
}

#[test]
fn tracker_record_keeps_portable_provenance_descriptor_and_last_seen_tick() {
    let mut tracker = TrackedObjectRegistry::new(99, 8).unwrap();
    let provenance = provenance(99, 9);
    let descriptor = descriptor();
    let receipt = tracker
        .observe(OrganismId(7), provenance, descriptor, Tick::new(44))
        .unwrap();
    let record = tracker
        .record(OrganismId(7), receipt.tracked_object_id)
        .unwrap();
    assert_eq!(record.tracking_provenance, provenance);
    assert_eq!(record.tracking_key, provenance.canonical_key());
    assert_eq!(record.stable_physical_descriptor, descriptor);
    assert_eq!(record.last_seen_tick, Tick::new(44));
}

#[test]
fn world_save_roundtrip_preserves_grounded_physics_and_portable_tracking_key() {
    let (world, cyan, _) = two_physical_object_fixture();
    let before = world.entity(cyan).unwrap().clone();
    let save = PortableSaveFile::from_headless_world(
        "grounded-world-roundtrip",
        &world,
        RuntimeConfig::deterministic_default(4_303, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let loaded = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap()).unwrap();
    let restored = loaded.restore_headless_world().unwrap();
    let after = restored.entity(cyan).unwrap();

    assert_eq!(after.grounded_physical, before.grounded_physical);
    assert_eq!(after.tracking_provenance, before.tracking_provenance);
    assert_eq!(after.tracking_key, before.tracking_key);
    assert_eq!(
        after.tracking_provenance.canonical_key(),
        after.tracking_key
    );
}

#[test]
fn grounded_physical_state_participates_in_the_world_determinism_signature() {
    let (mut world, food_id, _) = two_physical_object_fixture();
    let before = world.stable_signature();

    world
        .set_grounded_physical_properties(food_id, amber_sweet())
        .unwrap();

    assert_ne!(world.stable_signature(), before);
}

#[test]
fn tracked_object_id_zero_remains_reserved() {
    assert_eq!(TrackedObjectId(0).raw(), 0);
}
