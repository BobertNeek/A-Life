use alife_core::{
    ActionId, ActionKind, ActionTarget, BrainScaleTier, Confidence, DurationTicks, GenomeId,
    HomeostaticSnapshot, Intensity, OrganismId, PhysicalContactKind, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, LearningTraceSaveSummary,
        PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary,
    },
    EcologyConfig, EcologyZoneId, HeadlessScenarioBuilder, HeadlessWorldCommand,
    ResourceSpawnPolicy, TerrainZoneKind,
};

fn organism() -> OrganismId {
    OrganismId(707)
}

fn pos(x: f32, y: f32) -> Vec3f {
    Vec3f::new(x, y, 0.0)
}

fn move_to(position: Vec3f) -> alife_core::ActionCommand {
    alife_core::ActionCommand::structured(
        organism(),
        ActionId(1),
        ActionKind::Move,
        ActionTarget::new(None, Some(position)),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

fn ecology_world() -> alife_world::HeadlessWorld {
    HeadlessScenarioBuilder::new(7_070)
        .agent("creature", organism(), pos(0.0, 0.0))
        .food("berry", pos(0.8, 0.0), 0.7)
        .hazard("bramble", pos(4.0, 0.0), 0.25)
        .terrain_zone(
            1,
            "meadow",
            TerrainZoneKind::Meadow,
            pos(0.0, 0.0),
            3.0,
            0.8,
            0.0,
        )
        .terrain_zone(
            2,
            "ash-field",
            TerrainZoneKind::HazardField,
            pos(4.0, 0.0),
            2.0,
            0.1,
            0.65,
        )
        .track_resource("berry", 1, 2, 4)
        .build()
        .unwrap()
}

#[test]
fn resource_regrowth_is_deterministic_and_preserves_bounds() {
    let mut first = ecology_world();
    let mut second = ecology_world();
    let berry = first.entity_id("berry").unwrap();

    first
        .apply_command(&HeadlessWorldCommand::eat(organism(), berry).unwrap())
        .unwrap();
    second
        .apply_command(&HeadlessWorldCommand::eat(organism(), berry).unwrap())
        .unwrap();
    assert!(first.entity(berry).unwrap().is_consumed());

    first.advance_tick();
    second.advance_tick();
    assert!(first.entity(berry).unwrap().is_consumed());

    first.advance_tick();
    second.advance_tick();
    assert!(!first.entity(berry).unwrap().is_consumed());
    assert_eq!(first.stable_signature(), second.stable_signature());
    assert_eq!(first.ecology_metrics().resources_regrown, 1);
}

#[test]
fn hazard_zone_pressure_is_negative_bounded_and_visible_to_sensory_report() {
    let mut world = ecology_world();

    let before = world.sensory_report(organism(), Tick::ZERO).unwrap();
    assert_eq!(before.ecology.terrain_kind, Some(TerrainZoneKind::Meadow));
    assert_eq!(before.ecology.hazard_pressure, 0.0);

    let result = world.apply_command(&move_to(pos(3.0, 0.0))).unwrap();
    assert!(result.execution.succeeded);
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Moved
    );
    assert!(result.observation.reward_valence.raw() < 0.0);
    assert!(result.observation.pain_delta.raw() > 0.0);
    assert!(result.observation.pain_delta.raw() <= 1.0);

    let after = world.sensory_report(organism(), Tick::new(1)).unwrap();
    assert_eq!(
        after.ecology.terrain_kind,
        Some(TerrainZoneKind::HazardField)
    );
    assert!(after.ecology.hazard_pressure > 0.6);
    after.ecology.validate().unwrap();
}

#[test]
fn spawn_policy_and_world_caps_are_deterministic_and_bounded() {
    let config = EcologyConfig {
        max_world_objects: 3,
        max_resource_records: 2,
        max_zones: 2,
        max_spawn_per_tick: 2,
        cycle_length_ticks: 12,
    };
    let mut first = HeadlessScenarioBuilder::new(8_080)
        .ecology_config(config)
        .agent("creature", organism(), pos(0.0, 0.0))
        .terrain_zone(
            1,
            "grove",
            TerrainZoneKind::Grove,
            pos(1.0, 0.0),
            2.0,
            0.9,
            0.0,
        )
        .resource_spawn_policy("seed-berry", 1, 1, 4, 0.4)
        .build()
        .unwrap();
    let mut second = first.clone();

    let a = first.advance_ecology();
    let b = second.advance_ecology();
    assert_eq!(a, b);
    assert_eq!(a.spawned_labels, vec!["seed-berry-0".to_string()]);
    assert!(first.entity_id("seed-berry-0").is_some());

    first.advance_tick();
    second.advance_tick();
    first.advance_tick();
    second.advance_tick();
    assert_eq!(first.stable_signature(), second.stable_signature());
    assert!(first.stable_signature().len() <= config.max_world_objects);
    assert!(first.ecology_metrics().cap_rejections > 0);
}

#[test]
fn save_load_preserves_ecology_state_and_resource_lifecycle() {
    let mut world = ecology_world();
    let berry = world.entity_id("berry").unwrap();
    world
        .apply_command(&HeadlessWorldCommand::eat(organism(), berry).unwrap())
        .unwrap();
    world.advance_tick();

    let save = PortableSaveFile::from_headless_world(
        "g07-ecology-save",
        &world,
        RuntimeConfig::deterministic_default(7_070, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![fixture_creature()],
    )
    .unwrap();
    save.validate_with_asset_root(std::env::temp_dir()).unwrap();
    let json = save.to_json_string_pretty().unwrap();
    assert!(json.contains("\"ecology\""));

    let loaded = PortableSaveFile::from_json_str(&json).unwrap();
    let restored = loaded.restore_headless_world().unwrap();
    assert_eq!(restored.stable_signature(), world.stable_signature());
    assert_eq!(restored.ecology().resources.len(), 1);
    assert_eq!(
        restored.ecology().resources[0].consumed_at_tick,
        Some(Tick::ZERO)
    );
    assert!(restored.entity(berry).unwrap().is_consumed());
}

#[test]
fn invalid_or_unsupported_ecology_inputs_are_rejected() {
    let mut world = ecology_world();
    assert!(world
        .add_resource_spawn_policy(ResourceSpawnPolicy {
            label_prefix: String::new(),
            zone_id: EcologyZoneId(1),
            interval_ticks: 1,
            max_active: 1,
            nutrition: 0.5,
            next_spawn_tick: Tick::ZERO,
            spawned_count: 0,
        })
        .is_err());
    assert!(world
        .track_resource_lifecycle(WorldEntityId(999), EcologyZoneId(1), 1, 1)
        .is_err());
    assert!(world
        .track_resource_lifecycle(world.entity_id("bramble").unwrap(), EcologyZoneId(1), 1, 1,)
        .is_err());
}

fn fixture_creature() -> CreatureSaveState {
    CreatureSaveState {
        organism_id: organism(),
        genome_id: GenomeId(707),
        brain_class: BrainScaleTier::Nano512,
        development_tick: Tick::ZERO,
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::ZERO,
            homeostasis: HomeostaticSnapshot::baseline(Tick::ZERO),
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["g07 ecology fixture".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: None,
        },
        gpu_brain: None,
    }
}
