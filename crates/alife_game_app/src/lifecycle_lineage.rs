//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureLifeStage {
    Hatchling,
    Juvenile,
    Adult,
    Elder,
    Dead,
}

impl CreatureLifeStage {
    pub fn from_age(age_ticks: Tick, alive: bool) -> Self {
        if !alive {
            return Self::Dead;
        }
        match age_ticks.raw() {
            0..=1 => Self::Hatchling,
            2..=3 => Self::Juvenile,
            4..=8 => Self::Adult,
            _ => Self::Elder,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hatchling => "hatchling",
            Self::Juvenile => "juvenile",
            Self::Adult => "adult",
            Self::Elder => "elder",
            Self::Dead => "dead",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleEventKind {
    Aged,
    Birth,
    Death,
    ReproductionBlocked,
}

impl LifecycleEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aged => "aged",
            Self::Birth => "birth",
            Self::Death => "death",
            Self::ReproductionBlocked => "reproduction-blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCreatureConfig {
    pub organism_id: OrganismId,
    pub brain_tier: BrainScaleTier,
    pub label: &'static str,
    pub position: Vec3f,
    pub social_affinity: f32,
    pub homeostasis: HomeostaticSnapshot,
    pub initial_age_ticks: Tick,
    pub generation: u32,
}

impl LifecycleCreatureConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.label.is_empty() || self.generation > 64 {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        if !self.social_affinity.is_finite() || !(-1.0..=1.0).contains(&self.social_affinity) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.homeostasis.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleLoopConfig {
    pub seed: u64,
    pub lineage_id: LineageId,
    pub population_cap: usize,
    pub creatures: Vec<LifecycleCreatureConfig>,
    pub generated_weight_asset_id: Option<String>,
    pub logging_enabled: bool,
}

impl LifecycleLoopConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.lineage_id.validate()?;
        if self.population_cap < 2 || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.creatures.len() < 2 || self.creatures.len() > self.population_cap {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut ids = Vec::with_capacity(self.creatures.len());
        let mut labels = Vec::with_capacity(self.creatures.len());
        for creature in &self.creatures {
            creature.validate()?;
            ids.push(creature.organism_id.raw());
            labels.push(creature.label);
        }
        ids.sort_unstable();
        ids.dedup();
        labels.sort_unstable();
        labels.dedup();
        if ids.len() != self.creatures.len() || labels.len() != self.creatures.len() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self
            .generated_weight_asset_id
            .as_ref()
            .is_some_and(|asset| asset.is_empty() || asset.len() > 96)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn lineage_smoke() -> Result<Self, ScaffoldContractError> {
        let mut alpha = HomeostaticSnapshot::baseline(Tick::ZERO);
        alpha.drives.brain_atp = 0.84;
        alpha.drives.reproductive_drive = 0.82;
        alpha.drives.loneliness = 0.38;
        alpha.validate_contract()?;

        let mut beta = HomeostaticSnapshot::baseline(Tick::ZERO);
        beta.drives.brain_atp = 0.80;
        beta.drives.reproductive_drive = 0.78;
        beta.drives.loneliness = 0.36;
        beta.validate_contract()?;

        let mut elder = HomeostaticSnapshot::baseline(Tick::ZERO);
        elder.drives.brain_atp = 0.03;
        elder.drives.reproductive_drive = 0.05;
        elder.drives.fatigue = 0.92;
        elder.validate_contract()?;

        let config = Self {
            seed: 9_090,
            lineage_id: LineageId(9_090),
            population_cap: 4,
            logging_enabled: true,
            generated_weight_asset_id: Some("g09-tiny-birth-weight-asset".to_string()),
            creatures: vec![
                LifecycleCreatureConfig {
                    organism_id: OrganismId(901),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-alpha",
                    position: Vec3f::ZERO,
                    social_affinity: 0.66,
                    homeostasis: alpha,
                    initial_age_ticks: Tick::new(5),
                    generation: 0,
                },
                LifecycleCreatureConfig {
                    organism_id: OrganismId(902),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-beta",
                    position: Vec3f::new(1.0, 0.0, 0.0),
                    social_affinity: 0.64,
                    homeostasis: beta,
                    initial_age_ticks: Tick::new(5),
                    generation: 0,
                },
                LifecycleCreatureConfig {
                    organism_id: OrganismId(903),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-elder",
                    position: Vec3f::new(-1.0, 0.0, 0.0),
                    social_affinity: 0.10,
                    homeostasis: elder,
                    initial_age_ticks: Tick::new(10),
                    generation: 0,
                },
            ],
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleLineageRecord {
    pub offspring_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
    pub birth_tick: Tick,
    pub birth_weight_asset_id: Option<String>,
    pub lamarckian_enabled: bool,
    pub inherited_lifetime_state: bool,
}

impl LifecycleLineageRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.offspring_genome_id.validate()?;
        self.lineage_id.validate()?;
        if self.parent_genome_ids.len() != 2
            || self.generation == 0
            || self.lamarckian_enabled
            || self.inherited_lifetime_state
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.offspring_genome_id.raw(),
            self.parent_genome_ids
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join("+"),
            self.lineage_id.raw(),
            self.generation,
            self.birth_tick.raw(),
            self.birth_weight_asset_id.as_deref().unwrap_or("none"),
            self.inherited_lifetime_state
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleCreatureRecord {
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub label: String,
    pub genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
    pub age_ticks: Tick,
    pub life_stage: CreatureLifeStage,
    pub alive: bool,
    pub brain_atp: f32,
    pub reproductive_drive: f32,
    pub birth_weight_asset_id: Option<String>,
    pub genetic_prior_seed: u64,
    pub lamarckian_enabled: bool,
    pub inherited_lifetime_state: bool,
    pub death_reason: Option<String>,
}

impl LifecycleCreatureRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        self.genome_id.validate()?;
        self.lineage_id.validate()?;
        if self.label.is_empty()
            || !self.brain_atp.is_finite()
            || !self.reproductive_drive.is_finite()
            || !(0.0..=1.0).contains(&self.brain_atp)
            || !(0.0..=1.0).contains(&self.reproductive_drive)
            || self.genetic_prior_seed == 0
            || self.lamarckian_enabled
            || self.inherited_lifetime_state
            || self.life_stage != CreatureLifeStage::from_age(self.age_ticks, self.alive)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        if !self.alive && self.death_reason.is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{:.2}:{:.2}:{}:{}",
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.label,
            self.genome_id.raw(),
            self.lineage_id.raw(),
            self.generation,
            self.age_ticks.raw(),
            self.life_stage.label(),
            self.brain_atp,
            self.reproductive_drive,
            self.alive,
            self.birth_weight_asset_id.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleEventRecord {
    pub kind: LifecycleEventKind,
    pub tick: Tick,
    pub organism_id: OrganismId,
    pub stable_id: Option<WorldEntityId>,
    pub message: String,
}

impl LifecycleEventRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if let Some(stable_id) = self.stable_id {
            stable_id.validate()?;
        }
        if self.message.is_empty() || self.message.len() > 160 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}",
            self.kind.label(),
            self.tick.raw(),
            self.organism_id.raw(),
            self.stable_id.map(|id| id.raw()),
            self.message
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub population_cap: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub records: Vec<LifecycleCreatureRecord>,
    pub lineages: Vec<LifecycleLineageRecord>,
}

impl LifecycleSaveState {
    pub fn from_summary(summary: &LifecycleLineageSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        Ok(Self {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: summary.seed,
            population_cap: summary.population_cap,
            selected_stable_id: summary.selected_stable_id,
            records: summary.creatures.clone(),
            lineages: summary.lineage_records.clone(),
        })
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(json: &str) -> Result<Self, GameAppShellError> {
        let state = serde_json::from_str::<Self>(json)?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G09_LIFECYCLE_SCHEMA
            || self.schema_version != G09_LIFECYCLE_SCHEMA_VERSION
            || self.population_cap < 2
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.records.is_empty()
            || self.records.len() > self.population_cap
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for record in &self.records {
            record.validate()?;
        }
        for lineage in &self.lineages {
            lineage.validate()?;
        }
        if let Some(selected) = self.selected_stable_id {
            selected.validate()?;
            if !self
                .records
                .iter()
                .any(|record| record.alive && record.stable_id == selected)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.population_cap,
            self.selected_stable_id
                .map(|id| id.raw().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.records
                .iter()
                .map(LifecycleCreatureRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleInspectorLine {
    pub stable_id: WorldEntityId,
    pub organism_id: OrganismId,
    pub label: String,
    pub life_stage: CreatureLifeStage,
    pub lineage_label: String,
    pub selected: bool,
}

impl LifecycleInspectorLine {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.stable_id.validate()?;
        self.organism_id.validate()?;
        if self.label.is_empty() || self.lineage_label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.stable_id.raw(),
            self.organism_id.raw(),
            self.label,
            self.life_stage.label(),
            self.lineage_label,
            self.selected
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleMetrics {
    pub initial_population: usize,
    pub living_population: usize,
    pub population_cap: usize,
    pub births: usize,
    pub deaths: usize,
    pub reproduction_blocked_count: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
}

impl LifecycleMetrics {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.initial_population < 2
            || self.living_population > self.population_cap
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.births > self.population_cap
            || self.sealed_patch_count < self.initial_population
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleLineageSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub population_cap: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub creatures: Vec<LifecycleCreatureRecord>,
    pub lineage_records: Vec<LifecycleLineageRecord>,
    pub events: Vec<LifecycleEventRecord>,
    pub inspector_lines: Vec<LifecycleInspectorLine>,
    pub save_roundtrip_signature: String,
    pub world_signature: Vec<String>,
    pub metrics: LifecycleMetrics,
}

impl LifecycleLineageSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G09_LIFECYCLE_SCHEMA
            || self.schema_version != G09_LIFECYCLE_SCHEMA_VERSION
            || self.population_cap < 2
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.creatures.is_empty()
            || self.inspector_lines.is_empty()
            || self.world_signature.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.metrics.validate()?;
        for creature in &self.creatures {
            creature.validate()?;
        }
        for lineage in &self.lineage_records {
            lineage.validate()?;
        }
        for event in &self.events {
            event.validate()?;
        }
        for line in &self.inspector_lines {
            line.validate()?;
        }
        if let Some(selected) = self.selected_stable_id {
            selected.validate()?;
            if !self
                .creatures
                .iter()
                .any(|record| record.alive && record.stable_id == selected)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if self
            .creatures
            .iter()
            .any(|record| record.lamarckian_enabled || record.inherited_lifetime_state)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let save = LifecycleSaveState {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            records: self.creatures.clone(),
            lineages: self.lineage_records.clone(),
        };
        save.validate()?;
        if save.signature_line() != self.save_roundtrip_signature {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.population_cap,
            self.selected_stable_id
                .map(|id| id.raw().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.creatures
                .iter()
                .map(LifecycleCreatureRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.lineage_records
                .iter()
                .map(LifecycleLineageRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.events
                .iter()
                .map(LifecycleEventRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.save_roundtrip_signature
        )
    }
}

#[derive(Debug)]
struct LifecycleCreatureRuntime {
    organism_id: OrganismId,
    label: String,
    stable_id: WorldEntityId,
    mind: CreatureMind,
    genome: BrainGenome,
    generation: u32,
    age_ticks: Tick,
    alive: bool,
    birth_weight_asset_id: Option<String>,
    death_reason: Option<String>,
}

#[derive(Debug)]
pub struct LifecycleLiveLoop {
    seed: u64,
    lineage_id: LineageId,
    population_cap: usize,
    generated_weight_asset_id: Option<String>,
    logging_enabled: bool,
    selected_stable_id: Option<WorldEntityId>,
    initial_population: usize,
    harness: HeadlessBrainHarness,
    creatures: Vec<LifecycleCreatureRuntime>,
    lineage_records: Vec<LifecycleLineageRecord>,
    events: Vec<LifecycleEventRecord>,
    reproduction_blocked_count: usize,
}

impl LifecycleLiveLoop {
    pub fn from_config(config: LifecycleLoopConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        let mut builder = HeadlessScenarioBuilder::new(config.seed)
            .food("lineage-berry", Vec3f::new(2.0, 0.0, 0.0), 0.52)
            .obstacle("lineage-stone", Vec3f::new(-2.0, 0.0, 0.0), 0.55);
        for creature in &config.creatures {
            builder = builder.social_agent(
                creature.label,
                creature.organism_id,
                creature.position,
                creature.social_affinity,
            );
        }
        let world = builder.build()?;
        let mut creatures = Vec::with_capacity(config.population_cap);
        for creature in &config.creatures {
            let stable_id =
                world
                    .entity_id(creature.label)
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "G09 lifecycle creature label must map to a stable world ID",
                    })?;
            let species_seed = config.seed.saturating_add(creature.organism_id.raw());
            let mut genome =
                BrainGenome::scaffold(species_seed, creature.brain_tier.default_class_id());
            genome.lineage_id = Some(config.lineage_id);
            genome.validate_contract()?;
            let mut mind = CreatureMind::scaffold(
                creature.organism_id,
                creature.brain_tier,
                species_seed,
                Tick::ZERO,
            )?;
            *mind.homeostasis_mut() = creature.homeostasis;
            mind.homeostasis().validate_contract()?;
            creatures.push(LifecycleCreatureRuntime {
                organism_id: creature.organism_id,
                label: creature.label.to_string(),
                stable_id,
                mind,
                genome,
                generation: creature.generation,
                age_ticks: creature.initial_age_ticks,
                alive: true,
                birth_weight_asset_id: None,
                death_reason: None,
            });
        }
        creatures.sort_by_key(|creature| creature.organism_id.raw());
        Ok(Self {
            seed: config.seed,
            lineage_id: config.lineage_id,
            population_cap: config.population_cap,
            generated_weight_asset_id: config.generated_weight_asset_id,
            logging_enabled: config.logging_enabled,
            selected_stable_id: creatures.first().map(|creature| creature.stable_id),
            initial_population: creatures.len(),
            harness: HeadlessBrainHarness::new(world),
            creatures,
            lineage_records: Vec::new(),
            events: Vec::new(),
            reproduction_blocked_count: 0,
        })
    }

    pub fn run_lifecycle_once(&mut self) -> Result<LifecycleLineageSummary, GameAppShellError> {
        self.tick_living_creatures()?;
        self.age_living_creatures()?;
        self.remove_dead_creatures()?;
        self.try_reproduce()?;
        self.build_summary()
    }

    fn tick_living_creatures(&mut self) -> Result<(), GameAppShellError> {
        for index in 0..self.creatures.len() {
            if !self.creatures[index].alive {
                continue;
            }
            let tick_before = self.creatures[index].mind.current_tick();
            let input = BrainTickInput::new(
                tick_before,
                vec![proposal(
                    ActionKind::Idle.canonical_id(),
                    ActionKind::Idle,
                    None,
                    None,
                    0.80,
                    0.90,
                    0.0,
                )?],
            )
            .with_pack_experience(self.logging_enabled)
            .with_action_duration(DurationTicks::new(1));
            let tick = self
                .harness
                .tick_mind(&mut self.creatures[index].mind, input);
            if tick.brain.experience_patch.is_none() {
                return Err(GameAppShellError::Core(
                    ScaffoldContractError::MissingPhaseData,
                ));
            }
        }
        Ok(())
    }

    fn age_living_creatures(&mut self) -> Result<(), ScaffoldContractError> {
        for creature in &mut self.creatures {
            if !creature.alive {
                continue;
            }
            creature.age_ticks = Tick::new(creature.age_ticks.raw().saturating_add(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::Aged,
                tick: creature.mind.current_tick(),
                organism_id: creature.organism_id,
                stable_id: Some(creature.stable_id),
                message: format!(
                    "{} is now {}",
                    creature.label,
                    CreatureLifeStage::from_age(creature.age_ticks, true).label()
                ),
            });
        }
        Ok(())
    }

    fn remove_dead_creatures(&mut self) -> Result<(), GameAppShellError> {
        for index in 0..self.creatures.len() {
            if !self.creatures[index].alive {
                continue;
            }
            let brain_atp = self.creatures[index].mind.homeostasis().drives.brain_atp;
            if brain_atp > 0.05 && self.creatures[index].age_ticks.raw() < 12 {
                continue;
            }
            let stable_id = self.creatures[index].stable_id;
            let organism_id = self.creatures[index].organism_id;
            let label = self.creatures[index].label.clone();
            self.harness.remove_agent_entity(stable_id)?;
            self.creatures[index].alive = false;
            self.creatures[index].death_reason = Some(if brain_atp <= 0.05 {
                "energy-failure".to_string()
            } else {
                "old-age".to_string()
            });
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::Death,
                tick: self.creatures[index].mind.current_tick(),
                organism_id,
                stable_id: Some(stable_id),
                message: format!(
                    "{} removed by {}",
                    label,
                    self.creatures[index]
                        .death_reason
                        .as_deref()
                        .unwrap_or("unknown")
                ),
            });
        }
        self.selected_stable_id = self.selected_stable_id.and_then(|selected| {
            self.creatures
                .iter()
                .any(|creature| creature.alive && creature.stable_id == selected)
                .then_some(selected)
        });
        if self.selected_stable_id.is_none() {
            self.selected_stable_id = self
                .creatures
                .iter()
                .filter(|creature| creature.alive)
                .min_by_key(|creature| creature.organism_id.raw())
                .map(|creature| creature.stable_id);
        }
        Ok(())
    }

    fn try_reproduce(&mut self) -> Result<(), GameAppShellError> {
        let living = self
            .creatures
            .iter()
            .filter(|creature| creature.alive)
            .count();
        if living >= self.population_cap {
            self.reproduction_blocked_count = self.reproduction_blocked_count.saturating_add(1);
            let organism_id = self
                .creatures
                .iter()
                .find(|creature| creature.alive)
                .map(|creature| creature.organism_id)
                .unwrap_or(OrganismId(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::ReproductionBlocked,
                tick: self.harness.world().tick(),
                organism_id,
                stable_id: self.selected_stable_id,
                message: "population cap reached".to_string(),
            });
            return Ok(());
        }

        let parents = self
            .creatures
            .iter()
            .enumerate()
            .filter(|(_, creature)| {
                creature.alive
                    && CreatureLifeStage::from_age(creature.age_ticks, true)
                        == CreatureLifeStage::Adult
                    && creature.mind.homeostasis().drives.reproductive_drive >= 0.70
                    && creature.mind.homeostasis().drives.brain_atp >= 0.25
            })
            .take(2)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if parents.len() < 2 {
            self.reproduction_blocked_count = self.reproduction_blocked_count.saturating_add(1);
            let organism_id = self
                .creatures
                .iter()
                .find(|creature| creature.alive)
                .map(|creature| creature.organism_id)
                .unwrap_or(OrganismId(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::ReproductionBlocked,
                tick: self.harness.world().tick(),
                organism_id,
                stable_id: self.selected_stable_id,
                message: "no eligible adult pair".to_string(),
            });
            return Ok(());
        }

        let parent_a = parents[0];
        let parent_b = parents[1];
        for parent in [parent_a, parent_b] {
            let homeostasis = self.creatures[parent].mind.homeostasis_mut();
            homeostasis.drives.brain_atp = (homeostasis.drives.brain_atp - 0.12).max(0.0);
            homeostasis.drives.reproductive_drive =
                (homeostasis.drives.reproductive_drive - 0.35).max(0.0);
            homeostasis.validate_contract()?;
        }

        let child_number = self.creatures.len() as u64 + 1;
        let child_organism = OrganismId(self.seed.saturating_add(900).saturating_add(child_number));
        let child_label = format!("lineage-child-{}", child_organism.raw());
        let parent_position_a = self
            .harness
            .world()
            .entity(self.creatures[parent_a].stable_id)
            .map(|object| object.position)
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "parent A stable ID must exist before reproduction",
            })?;
        let parent_position_b = self
            .harness
            .world()
            .entity(self.creatures[parent_b].stable_id)
            .map(|object| object.position)
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "parent B stable ID must exist before reproduction",
            })?;
        let child_position = Vec3f::new(
            (parent_position_a.x + parent_position_b.x) * 0.5,
            (parent_position_a.y + parent_position_b.y) * 0.5,
            (parent_position_a.z + parent_position_b.z) * 0.5 + 0.5,
        );
        let stable_id =
            self.harness
                .spawn_social_agent(&child_label, child_organism, child_position, 0.50)?;
        let child_seed = deterministic_child_seed(
            self.seed,
            self.creatures[parent_a].genome.id,
            self.creatures[parent_b].genome.id,
            child_organism,
        );
        let mut genome =
            BrainGenome::scaffold(child_seed, BrainScaleTier::Nano512.default_class_id());
        genome.parent_genome_ids = vec![
            self.creatures[parent_a].genome.id,
            self.creatures[parent_b].genome.id,
        ];
        genome.lineage_id = Some(self.lineage_id);
        genome.validate_contract()?;

        let mut mind = CreatureMind::scaffold(
            child_organism,
            BrainScaleTier::Nano512,
            child_seed,
            Tick::ZERO,
        )?;
        let mut child_homeostasis = HomeostaticSnapshot::baseline(Tick::ZERO);
        child_homeostasis.drives.reproductive_drive = 0.05;
        child_homeostasis.drives.brain_atp = 0.62;
        child_homeostasis.validate_contract()?;
        *mind.homeostasis_mut() = child_homeostasis;

        let generation = self.creatures[parent_a]
            .generation
            .max(self.creatures[parent_b].generation)
            + 1;
        let record = LifecycleLineageRecord {
            offspring_genome_id: genome.id,
            parent_genome_ids: genome.parent_genome_ids.clone(),
            lineage_id: self.lineage_id,
            generation,
            birth_tick: self.harness.world().tick(),
            birth_weight_asset_id: self.generated_weight_asset_id.clone(),
            lamarckian_enabled: genome.inheritance.lamarckian_weights_enabled,
            inherited_lifetime_state: genome.inheritance.inherit_lifetime_consolidation,
        };
        record.validate()?;
        self.lineage_records.push(record);
        self.events.push(LifecycleEventRecord {
            kind: LifecycleEventKind::Birth,
            tick: self.harness.world().tick(),
            organism_id: child_organism,
            stable_id: Some(stable_id),
            message: format!(
                "{} born from {}+{}",
                child_label,
                self.creatures[parent_a].organism_id.raw(),
                self.creatures[parent_b].organism_id.raw()
            ),
        });
        self.creatures.push(LifecycleCreatureRuntime {
            organism_id: child_organism,
            label: child_label,
            stable_id,
            mind,
            genome,
            generation,
            age_ticks: Tick::ZERO,
            alive: true,
            birth_weight_asset_id: self.generated_weight_asset_id.clone(),
            death_reason: None,
        });
        self.creatures
            .sort_by_key(|creature| creature.organism_id.raw());
        Ok(())
    }

    fn build_summary(&self) -> Result<LifecycleLineageSummary, GameAppShellError> {
        let creatures = self
            .creatures
            .iter()
            .map(|creature| {
                let homeostasis = creature.mind.homeostasis();
                let record = LifecycleCreatureRecord {
                    organism_id: creature.organism_id,
                    stable_id: creature.stable_id,
                    label: creature.label.clone(),
                    genome_id: creature.genome.id,
                    parent_genome_ids: creature.genome.parent_genome_ids.clone(),
                    lineage_id: creature
                        .genome
                        .lineage_id
                        .ok_or(GameAppShellError::Core(ScaffoldContractError::InvalidId))?,
                    generation: creature.generation,
                    age_ticks: creature.age_ticks,
                    life_stage: CreatureLifeStage::from_age(creature.age_ticks, creature.alive),
                    alive: creature.alive,
                    brain_atp: homeostasis.drives.brain_atp,
                    reproductive_drive: homeostasis.drives.reproductive_drive,
                    birth_weight_asset_id: creature.birth_weight_asset_id.clone(),
                    genetic_prior_seed: creature.genome.genetic_prior_seed,
                    lamarckian_enabled: creature.genome.inheritance.lamarckian_weights_enabled,
                    inherited_lifetime_state: creature
                        .genome
                        .inheritance
                        .inherit_lifetime_consolidation,
                    death_reason: creature.death_reason.clone(),
                };
                record.validate()?;
                Ok(record)
            })
            .collect::<Result<Vec<_>, GameAppShellError>>()?;
        let inspector_lines = creatures
            .iter()
            .filter(|record| record.alive)
            .map(|record| {
                let line = LifecycleInspectorLine {
                    stable_id: record.stable_id,
                    organism_id: record.organism_id,
                    label: record.label.clone(),
                    life_stage: record.life_stage,
                    lineage_label: format!(
                        "lineage-{}-gen{}",
                        record.lineage_id.raw(),
                        record.generation
                    ),
                    selected: self.selected_stable_id == Some(record.stable_id),
                };
                line.validate()?;
                Ok(line)
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let save_preview = LifecycleSaveState {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            records: creatures.clone(),
            lineages: self.lineage_records.clone(),
        };
        let json = save_preview.to_json_string_pretty()?;
        let save_roundtrip = LifecycleSaveState::from_json_str(&json)?;
        let summary = LifecycleLineageSummary {
            schema: G09_LIFECYCLE_SCHEMA,
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            creatures,
            lineage_records: self.lineage_records.clone(),
            events: self.events.clone(),
            inspector_lines,
            save_roundtrip_signature: save_roundtrip.signature_line(),
            world_signature: self.harness.world().stable_signature(),
            metrics: LifecycleMetrics {
                initial_population: self.initial_population,
                living_population: self
                    .creatures
                    .iter()
                    .filter(|creature| creature.alive)
                    .count(),
                population_cap: self.population_cap,
                births: self.lineage_records.len(),
                deaths: self
                    .creatures
                    .iter()
                    .filter(|creature| !creature.alive)
                    .count(),
                reproduction_blocked_count: self.reproduction_blocked_count,
                sealed_patch_count: self.harness.telemetry().sealed_patches.len(),
                packed_record_count: self.harness.telemetry().packed_records.len(),
            },
        };
        summary.validate()?;
        Ok(summary)
    }
}

fn deterministic_child_seed(
    seed: u64,
    parent_a: GenomeId,
    parent_b: GenomeId,
    child: OrganismId,
) -> u64 {
    let mut value = seed
        ^ parent_a.raw().rotate_left(17)
        ^ parent_b.raw().rotate_right(11)
        ^ child.raw().wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 29;
    value.max(1)
}

pub fn run_lifecycle_lineage_smoke() -> Result<LifecycleLineageSummary, GameAppShellError> {
    let config = LifecycleLoopConfig::lineage_smoke()?;
    let mut live = LifecycleLiveLoop::from_config(config)?;
    live.run_lifecycle_once()
}
