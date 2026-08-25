use alife_core::{
    BrainCapacityClass, BrainScaleTier, FoundationGeneticIdentity, FoundationWeightAsset, GenomeId,
    LineageId, OrganismId, ScaffoldContractError, SensorProfile, Tick, Vec3f, WorldEntityId,
};

use crate::{
    CreatureAppearanceGenome, CreatureMindSaveSummary, CreatureSaveState, EcologyZoneId,
    HabitatAuthority, HabitatId, HeadlessWorld, LearningTraceSaveSummary, PortableAssetDigest,
    TerrainZone, TerrainZoneKind, WeightLayerSaveSummary, WorldEditorSpawnSpec, WorldObjectKind,
    WorldOrganismRecord,
};

pub const PHASE3_NEW_GAME_SCHEMA_VERSION: u16 = 1;
pub const PHASE3_DEFAULT_POPULATION: u16 = 6;
pub const PHASE3_MIN_POPULATION: u16 = 4;
pub const PHASE3_MAX_POPULATION: u16 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalNewGameConfig {
    pub schema_version: u16,
    pub world_seed: u64,
    pub founder_count: u16,
    pub brain_class: BrainScaleTier,
    pub sensor_profile: SensorProfile,
}

impl CanonicalNewGameConfig {
    pub fn phase3(world_seed: u64, founder_count: u16) -> Result<Self, ScaffoldContractError> {
        if world_seed == 0
            || !(PHASE3_MIN_POPULATION..=PHASE3_MAX_POPULATION).contains(&founder_count)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(Self {
            schema_version: PHASE3_NEW_GAME_SCHEMA_VERSION,
            world_seed,
            founder_count,
            brain_class: BrainScaleTier::Nano512,
            sensor_profile: SensorProfile::GroundedObjectSlotsV1,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFounderReceipt {
    pub organism_id: OrganismId,
    pub world_entity_id: WorldEntityId,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
    pub world_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNewGameReceipt {
    pub schema_version: u16,
    pub world_seed: u64,
    pub requested_population: u16,
    pub founders: Vec<CanonicalFounderReceipt>,
}

#[derive(Debug, Clone)]
pub struct CanonicalNewGame {
    pub world: HeadlessWorld,
    pub creatures: Vec<CreatureSaveState>,
    pub receipt: CanonicalNewGameReceipt,
}

pub fn create_canonical_new_game(
    config: &CanonicalNewGameConfig,
    foundation: &FoundationWeightAsset,
) -> Result<CanonicalNewGame, ScaffoldContractError> {
    validate_phase3_inputs(config, foundation)?;

    let manifest = foundation.manifest();
    let foundation_identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        u16::try_from(manifest.foundation_version().raw())
            .map_err(|_| ScaffoldContractError::InvalidId)?,
        manifest.compatibility_family_id().raw(),
        BrainCapacityClass::N512_ID,
    )?;
    let mut world = HeadlessWorld::new(config.world_seed);
    let mut habitats = HabitatAuthority::default();
    let mut creatures = Vec::with_capacity(usize::from(config.founder_count));
    let mut founders = Vec::with_capacity(usize::from(config.founder_count));

    for slot in 0..config.founder_count {
        let ordinal = u64::from(slot) + 1;
        let organism_id = OrganismId(ordinal);
        let founder_seed = config
            .world_seed
            .checked_mul(16)
            .and_then(|seed| seed.checked_add(ordinal))
            .ok_or(ScaffoldContractError::InvalidId)?;
        let world_label = format!("founder-{ordinal:02}");
        let position = founder_position(slot);
        let genome =
            alife_core::CreatureGenome::early_mammal_founder(founder_seed, foundation_identity)?;
        let phenotype = genome.express()?;
        let world_entity_id = world.spawn_social_agent(&world_label, organism_id, position, 0.0)?;
        let record = WorldOrganismRecord::newborn(
            organism_id,
            world_entity_id,
            genome,
            phenotype,
            Tick::ZERO,
        )
        .map_err(|_| ScaffoldContractError::InvalidId)?;
        let creature = initial_creature_save(&record, slot, founder_seed)?;
        let receipt = CanonicalFounderReceipt {
            organism_id,
            world_entity_id,
            genome_id: record.genome().id,
            lineage_id: record.genome().lineage_id,
            world_label,
        };
        world.register_organism_record(record)?;
        habitats
            .register_creature(organism_id, HabitatId::DEFAULT_WILD, Tick::ZERO)
            .map_err(|_| ScaffoldContractError::InvalidId)?;
        creatures.push(creature);
        founders.push(receipt);
    }

    world
        .replace_habitat_authority(habitats)
        .map_err(|_| ScaffoldContractError::InvalidId)?;
    spawn_phase3_ecology(&mut world)?;

    Ok(CanonicalNewGame {
        world,
        creatures,
        receipt: CanonicalNewGameReceipt {
            schema_version: PHASE3_NEW_GAME_SCHEMA_VERSION,
            world_seed: config.world_seed,
            requested_population: config.founder_count,
            founders,
        },
    })
}

fn spawn_phase3_ecology(world: &mut HeadlessWorld) -> Result<(), ScaffoldContractError> {
    let meadow = EcologyZoneId(1);
    world.add_terrain_zone(TerrainZone::new(
        meadow,
        "founder-meadow",
        TerrainZoneKind::Meadow,
        Vec3f::ZERO,
        12.0,
        0.8,
        0.2,
    )?)?;

    let food_id = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "food-01".to_string(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: founder_position(0),
        nutrition: 0.65,
        hazard_pain: 0.0,
        radius: 0.45,
        token_id: None,
    })?;
    world.track_resource_lifecycle(food_id, meadow, 48, 240)?;

    world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "hazard-01".to_string(),
        kind: WorldObjectKind::Hazard,
        organism_id: None,
        position: founder_position(1),
        nutrition: 0.0,
        hazard_pain: 0.12,
        radius: 0.8,
        token_id: None,
    })?;
    for (label, position) in [
        ("obstacle-01", Vec3f::new(0.0, -7.0, 0.0)),
        ("obstacle-02", Vec3f::new(0.0, 7.0, 0.0)),
    ] {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: label.to_string(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 1.1,
            token_id: None,
        })?;
    }
    Ok(())
}

fn validate_phase3_inputs(
    config: &CanonicalNewGameConfig,
    foundation: &FoundationWeightAsset,
) -> Result<(), ScaffoldContractError> {
    let canonical = CanonicalNewGameConfig::phase3(config.world_seed, config.founder_count)?;
    if *config != canonical
        || foundation
            != &FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)?
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(())
}

fn founder_position(slot: u16) -> Vec3f {
    let angle = f32::from(slot) * core::f32::consts::TAU / f32::from(PHASE3_MAX_POPULATION);
    Vec3f::new(angle.cos() * 3.0, angle.sin() * 3.0, 0.0)
}

fn initial_creature_save(
    record: &WorldOrganismRecord,
    slot: u16,
    founder_seed: u64,
) -> Result<CreatureSaveState, ScaffoldContractError> {
    let biochemistry = record.biochemistry();
    let genome_bytes =
        serde_json::to_vec(record.genome()).map_err(|_| ScaffoldContractError::InvalidId)?;
    Ok(CreatureSaveState {
        organism_id: record.organism_id(),
        genome_id: record.genome().id,
        brain_class: BrainScaleTier::Nano512,
        development_tick: biochemistry.development.last_update_tick,
        appearance: CreatureAppearanceGenome::founder_for_species(
            u8::try_from(slot).map_err(|_| ScaffoldContractError::InvalidId)?,
            founder_seed,
        ),
        mind: CreatureMindSaveSummary {
            tick: biochemistry.tick,
            homeostasis: biochemistry.homeostasis,
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["canonical Phase 3 founder awaiting GPU admission".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: PortableAssetDigest::for_bytes(&genome_bytes).0,
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
        composite_genetics: None,
        lifetime_state_asset: None,
        gpu_brain: None,
    })
}
