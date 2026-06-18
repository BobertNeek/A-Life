//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationSocialEventKind {
    Vocalize,
    SocialApproach,
}

impl PopulationSocialEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vocalize => "vocalize",
            Self::SocialApproach => "social-approach",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopulationCreatureConfig {
    pub organism_id: OrganismId,
    pub brain_tier: BrainScaleTier,
    pub label: &'static str,
    pub position: Vec3f,
    pub social_affinity: f32,
    pub homeostasis: HomeostaticSnapshot,
}

impl PopulationCreatureConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.label.is_empty() {
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
pub struct PopulationLoopConfig {
    pub seed: u64,
    pub population_cap: usize,
    pub creatures: Vec<PopulationCreatureConfig>,
    pub rounds: u32,
    pub logging_enabled: bool,
}

impl PopulationLoopConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.population_cap == 0 || self.population_cap > G08_MAX_POPULATION_CAP {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.creatures.len() < 2 || self.creatures.len() > self.population_cap {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.rounds == 0 || self.rounds > 8 {
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
        Ok(())
    }

    pub fn two_creature_smoke() -> Result<Self, ScaffoldContractError> {
        let mut alpha = HomeostaticSnapshot::baseline(Tick::ZERO);
        alpha.drives.loneliness = 0.42;
        alpha.drives.curiosity = 0.62;
        alpha.drives.brain_atp = 0.72;
        alpha.validate_contract()?;

        let mut beta = HomeostaticSnapshot::baseline(Tick::ZERO);
        beta.drives.loneliness = 0.55;
        beta.drives.curiosity = 0.58;
        beta.drives.brain_atp = 0.70;
        beta.validate_contract()?;

        let config = Self {
            seed: 8_080,
            population_cap: 4,
            rounds: 2,
            logging_enabled: true,
            creatures: vec![
                PopulationCreatureConfig {
                    organism_id: OrganismId(801),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "alpha",
                    position: Vec3f::ZERO,
                    social_affinity: 0.65,
                    homeostasis: alpha,
                },
                PopulationCreatureConfig {
                    organism_id: OrganismId(802),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "beta",
                    position: Vec3f::new(1.0, 0.0, 0.0),
                    social_affinity: -0.70,
                    homeostasis: beta,
                },
            ],
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationTickRecord {
    pub round: u32,
    pub order_index: usize,
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub event_kind: PopulationSocialEventKind,
    pub tick_summary: LiveBrainTickSummary,
    pub social_agents_seen: usize,
    pub heard_tokens: usize,
    pub trust_cues_seen: usize,
    pub fear_cues_seen: usize,
    pub contacted_agents: usize,
    pub social_direct_action_count: usize,
}

impl PopulationTickRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        if self.social_direct_action_count != 0
            || self.order_index >= G08_MAX_POPULATION_CAP
            || !self.tick_summary.patch_sealed
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{:?}:{:?}:{}:{}:{}:{}:{}",
            self.round,
            self.order_index,
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.event_kind.label(),
            self.tick_summary.selected_action_kind,
            self.tick_summary.target_entity.map(|id| id.raw()),
            self.social_agents_seen,
            self.heard_tokens,
            self.trust_cues_seen,
            self.fear_cues_seen,
            self.contacted_agents
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationCreatureStatus {
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub label: String,
    pub position: Vec3f,
    pub last_action_kind: Option<ActionKind>,
    pub social_agents_seen: usize,
    pub heard_tokens: usize,
    pub visual: CreatureVisualSnapshot,
}

impl PopulationCreatureStatus {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        self.visual.validate()?;
        if self.visual.organism_id != self.organism_id || self.visual.stable_id != self.stable_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.2}:{:.2}:{:.2}:{:?}:{}:{}",
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.label,
            self.position.x,
            self.position.y,
            self.position.z,
            self.last_action_kind,
            self.social_agents_seen,
            self.heard_tokens
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationPerformanceMetrics {
    pub creature_count: usize,
    pub population_cap: usize,
    pub scheduler_steps: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub social_context_samples: usize,
    pub vocal_tokens_heard: usize,
    pub collision_feedback_count: usize,
    pub world_object_count: usize,
}

impl PopulationPerformanceMetrics {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.creature_count < 2
            || self.creature_count > self.population_cap
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.scheduler_steps == 0
            || self.sealed_patch_count < self.scheduler_steps
            || self.world_object_count < self.creature_count
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationSocialLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub creature_count: usize,
    pub population_cap: usize,
    pub schedule_order: Vec<OrganismId>,
    pub tick_records: Vec<PopulationTickRecord>,
    pub creature_status: Vec<PopulationCreatureStatus>,
    pub metrics: PopulationPerformanceMetrics,
    pub world_signature: Vec<String>,
}

impl PopulationSocialLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G08_POPULATION_SOCIAL_SCHEMA
            || self.schema_version != G08_POPULATION_SOCIAL_SCHEMA_VERSION
            || self.creature_count < 2
            || self.creature_count > self.population_cap
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.schedule_order.len() != self.creature_count
            || self.creature_status.len() != self.creature_count
            || self.tick_records.len() < self.creature_count
            || !self.tick_records.len().is_multiple_of(self.creature_count)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut order = self
            .schedule_order
            .iter()
            .map(|id| {
                id.validate()?;
                Ok(id.raw())
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let sorted = {
            let mut copy = order.clone();
            copy.sort_unstable();
            copy
        };
        if order != sorted {
            return Err(ScaffoldContractError::InvalidId);
        }
        order.dedup();
        if order.len() != self.schedule_order.len() {
            return Err(ScaffoldContractError::InvalidId);
        }
        for record in &self.tick_records {
            record.validate()?;
        }
        for status in &self.creature_status {
            status.validate()?;
        }
        self.metrics.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.creature_count,
            self.population_cap,
            self.schedule_order
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join(">"),
            self.tick_records
                .iter()
                .map(PopulationTickRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.creature_status
                .iter()
                .map(PopulationCreatureStatus::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug)]
struct PopulationCreatureRuntime {
    organism_id: OrganismId,
    label: String,
    stable_id: WorldEntityId,
    mind: CreatureMind,
    last_summary: Option<LiveBrainTickSummary>,
    last_social_agents_seen: usize,
    last_heard_tokens: usize,
}

#[derive(Debug)]
pub struct PopulationLiveLoop {
    population_cap: usize,
    logging_enabled: bool,
    harness: HeadlessBrainHarness,
    creatures: Vec<PopulationCreatureRuntime>,
}

impl PopulationLiveLoop {
    pub fn from_config(config: PopulationLoopConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        let mut builder = HeadlessScenarioBuilder::new(config.seed)
            .food("shared-berry", Vec3f::new(2.0, 0.0, 0.0), 0.45)
            .obstacle("social-rock", Vec3f::new(-2.0, 0.0, 0.0), 0.65);
        for creature in &config.creatures {
            builder = builder.social_agent(
                creature.label,
                creature.organism_id,
                creature.position,
                creature.social_affinity,
            );
        }
        let world = builder.build()?;
        let mut creatures = Vec::with_capacity(config.creatures.len());
        for creature in config.creatures {
            let stable_id =
                world
                    .entity_id(creature.label)
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "G08 population creature label must map to a stable world ID",
                    })?;
            let mut mind = CreatureMind::scaffold(
                creature.organism_id,
                creature.brain_tier,
                config.seed,
                Tick::ZERO,
            )?;
            *mind.homeostasis_mut() = creature.homeostasis;
            mind.homeostasis().validate_contract()?;
            creatures.push(PopulationCreatureRuntime {
                organism_id: creature.organism_id,
                label: creature.label.to_string(),
                stable_id,
                mind,
                last_summary: None,
                last_social_agents_seen: 0,
                last_heard_tokens: 0,
            });
        }
        creatures.sort_by_key(|creature| creature.organism_id.raw());
        Ok(Self {
            population_cap: config.population_cap,
            logging_enabled: config.logging_enabled,
            harness: HeadlessBrainHarness::new(world),
            creatures,
        })
    }

    pub fn run_rounds(
        &mut self,
        rounds: u32,
        seed: u64,
    ) -> Result<PopulationSocialLoopSummary, GameAppShellError> {
        if rounds == 0 || rounds > 8 || self.creatures.len() < 2 {
            return Err(GameAppShellError::Core(
                ScaffoldContractError::ScalarOutOfRange,
            ));
        }
        let mut records = Vec::with_capacity(rounds as usize * self.creatures.len());
        for round in 0..rounds {
            for order_index in 0..self.creatures.len() {
                let organism_id = self.creatures[order_index].organism_id;
                let stable_id = self.creatures[order_index].stable_id;
                let report = self
                    .harness
                    .world()
                    .sensory_report(organism_id, self.creatures[order_index].mind.current_tick())?;
                let social_agents_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .count();
                let heard_tokens = report
                    .core_snapshot
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .count();
                let trust_cues_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .filter(|agent| agent.affinity.raw() > 0.0)
                    .count();
                let fear_cues_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .filter(|agent| agent.affinity.raw() < 0.0)
                    .count();
                let (event_kind, proposals) =
                    self.scripted_population_proposals(round, order_index)?;
                let tick_before = self.creatures[order_index].mind.current_tick();
                let world_tick_before = self.harness.world().tick();
                let input = BrainTickInput::new(tick_before, proposals)
                    .with_pack_experience(self.logging_enabled)
                    .with_action_duration(DurationTicks::new(1));
                let tick = self
                    .harness
                    .tick_mind(&mut self.creatures[order_index].mind, input);
                let world_tick_after = self.harness.world().tick();
                let action_failure = tick
                    .action_result
                    .as_ref()
                    .and_then(|result| result.execution.failure);
                let contacted_agents = tick
                    .action_result
                    .as_ref()
                    .map(|result| {
                        let world = self.harness.world();
                        result
                            .touched_entities
                            .iter()
                            .filter(|id| {
                                world
                                    .entity(**id)
                                    .is_some_and(|object| object.kind == WorldObjectKind::Agent)
                            })
                            .count()
                    })
                    .unwrap_or(0);
                let summary = LiveBrainLoop::summarize_tick(
                    organism_id,
                    tick_before,
                    self.creatures[order_index].mind.current_tick(),
                    world_tick_before,
                    world_tick_after,
                    &tick.brain,
                    action_failure,
                    self.harness.telemetry().sealed_patches.len(),
                    self.harness.telemetry().packed_records.len(),
                );
                let record = PopulationTickRecord {
                    round,
                    order_index,
                    organism_id,
                    stable_id,
                    event_kind,
                    tick_summary: summary.clone(),
                    social_agents_seen,
                    heard_tokens,
                    trust_cues_seen,
                    fear_cues_seen,
                    contacted_agents,
                    social_direct_action_count: 0,
                };
                record.validate()?;
                self.creatures[order_index].last_summary = Some(summary);
                self.creatures[order_index].last_social_agents_seen = social_agents_seen;
                self.creatures[order_index].last_heard_tokens = heard_tokens;
                records.push(record);
            }
        }
        self.build_summary(seed, records)
    }

    fn scripted_population_proposals(
        &self,
        round: u32,
        order_index: usize,
    ) -> Result<(PopulationSocialEventKind, Vec<ActionProposal>), ScaffoldContractError> {
        let actor = &self.creatures[order_index];
        let partner_index = (order_index + 1) % self.creatures.len();
        let partner = &self.creatures[partner_index];
        if (round + order_index as u32).is_multiple_of(2) {
            Ok((
                PopulationSocialEventKind::Vocalize,
                vec![proposal(
                    ActionKind::Vocalize.canonical_id(),
                    ActionKind::Vocalize,
                    None,
                    None,
                    0.96,
                    0.97,
                    0.0,
                )?],
            ))
        } else {
            Ok((
                PopulationSocialEventKind::SocialApproach,
                vec![proposal(
                    ActionKind::Move.canonical_id(),
                    ActionKind::Move,
                    Some(partner.stable_id),
                    None,
                    0.94,
                    0.96,
                    distance_between_entities(
                        &self.harness.world(),
                        actor.stable_id,
                        partner.stable_id,
                    ),
                )?],
            ))
        }
    }

    fn build_summary(
        &self,
        seed: u64,
        records: Vec<PopulationTickRecord>,
    ) -> Result<PopulationSocialLoopSummary, GameAppShellError> {
        let schedule_order = self
            .creatures
            .iter()
            .map(|creature| creature.organism_id)
            .collect::<Vec<_>>();
        let statuses = self
            .creatures
            .iter()
            .map(|creature| {
                let object = self
                    .harness
                    .world()
                    .entity(creature.stable_id)
                    .cloned()
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "population stable creature ID must remain in the world",
                    })?;
                let target = creature
                    .last_summary
                    .as_ref()
                    .and_then(|summary| summary.target_entity);
                let target_position = target.and_then(|target_id| {
                    self.harness
                        .world()
                        .entity(target_id)
                        .map(|target| target.position)
                });
                let visual = creature_visual_snapshot_from_parts(
                    creature.organism_id,
                    creature.stable_id,
                    object.position,
                    target,
                    target_position,
                    creature.mind.homeostasis(),
                    creature.mind.sleep_state().phase,
                    creature
                        .last_summary
                        .as_ref()
                        .and_then(|summary| summary.selected_action_kind),
                )?;
                let status = PopulationCreatureStatus {
                    organism_id: creature.organism_id,
                    stable_id: creature.stable_id,
                    label: creature.label.clone(),
                    position: object.position,
                    last_action_kind: creature
                        .last_summary
                        .as_ref()
                        .and_then(|summary| summary.selected_action_kind),
                    social_agents_seen: creature.last_social_agents_seen,
                    heard_tokens: creature.last_heard_tokens,
                    visual,
                };
                status.validate()?;
                Ok(status)
            })
            .collect::<Result<Vec<_>, GameAppShellError>>()?;
        let metrics = PopulationPerformanceMetrics {
            creature_count: self.creatures.len(),
            population_cap: self.population_cap,
            scheduler_steps: records.len(),
            sealed_patch_count: self.harness.telemetry().sealed_patches.len(),
            packed_record_count: self.harness.telemetry().packed_records.len(),
            social_context_samples: records.iter().map(|record| record.social_agents_seen).sum(),
            vocal_tokens_heard: records.iter().map(|record| record.heard_tokens).sum(),
            collision_feedback_count: records
                .iter()
                .filter(|record| record.contacted_agents > 0)
                .count(),
            world_object_count: self.harness.world().stable_signature().len(),
        };
        let summary = PopulationSocialLoopSummary {
            schema: G08_POPULATION_SOCIAL_SCHEMA,
            schema_version: G08_POPULATION_SOCIAL_SCHEMA_VERSION,
            seed,
            creature_count: self.creatures.len(),
            population_cap: self.population_cap,
            schedule_order,
            tick_records: records,
            creature_status: statuses,
            metrics,
            world_signature: self.harness.world().stable_signature(),
        };
        summary.validate()?;
        Ok(summary)
    }
}

fn distance_between_entities(world: &HeadlessWorld, a: WorldEntityId, b: WorldEntityId) -> f32 {
    let Some(a) = world.entity(a) else {
        return 0.0;
    };
    let Some(b) = world.entity(b) else {
        return 0.0;
    };
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    let dz = a.position.z - b.position.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub fn run_population_social_loop_smoke() -> Result<PopulationSocialLoopSummary, GameAppShellError>
{
    let config = PopulationLoopConfig::two_creature_smoke()?;
    let seed = config.seed;
    let rounds = config.rounds;
    let mut live = PopulationLiveLoop::from_config(config)?;
    live.run_rounds(rounds, seed)
}
