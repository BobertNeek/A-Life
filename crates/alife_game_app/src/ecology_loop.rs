//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct EcologyIndicator {
    pub zone_id: EcologyZoneId,
    pub label: String,
    pub terrain_kind: TerrainZoneKind,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl EcologyIndicator {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.zone_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.resource_bias)?;
        NormalizedScalar::new(self.hazard_pressure)?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.3}:{:.3}",
            self.zone_id.raw(),
            self.label,
            self.terrain_kind.label(),
            self.resource_bias,
            self.hazard_pressure
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableEcologyLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub organism_id: OrganismId,
    pub tick_summaries: Vec<LiveBrainTickSummary>,
    pub ecology_indicators: Vec<EcologyIndicator>,
    pub metrics: EcologyMetrics,
    pub regrown_resource_id: Option<WorldEntityId>,
    pub spawned_labels: Vec<String>,
    pub hazard_tick: Tick,
    pub hazard_pain: f32,
    pub sensory_zone_label: Option<String>,
    pub world_signature: Vec<String>,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
}

impl PlayableEcologyLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema != G07_WORLD_ECOLOGY_SCHEMA
            || self.schema_version != G07_WORLD_ECOLOGY_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.tick_summaries.len() < 4
            || self.ecology_indicators.len() < 2
            || self.world_signature.len() > 64
            || self.sealed_patch_count < self.tick_summaries.len()
            || self.metrics.active_resources == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(id) = self.regrown_resource_id {
            id.validate()?;
        }
        NormalizedScalar::new(self.hazard_pain)?;
        if self.spawned_labels.iter().any(|label| label.is_empty()) {
            return Err(ScaffoldContractError::InvalidId);
        }
        for indicator in &self.ecology_indicators {
            indicator.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{:.3}:{}",
            self.schema_version,
            self.seed,
            self.organism_id.raw(),
            self.tick_summaries.len(),
            self.metrics.active_resources,
            self.metrics.resources_regrown,
            self.metrics.resources_spawned,
            self.hazard_pain,
            self.ecology_indicators
                .iter()
                .map(EcologyIndicator::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

pub fn run_world_ecology_loop_smoke() -> Result<PlayableEcologyLoopSummary, GameAppShellError> {
    const SEED: u64 = 7_070;
    let organism_id = OrganismId(707);
    let food_position = Vec3f::new(0.8, 0.0, 0.0);
    let hazard_position = Vec3f::new(4.0, 0.0, 0.0);
    let world = HeadlessScenarioBuilder::new(SEED)
        .agent("creature", organism_id, Vec3f::ZERO)
        .food("berry", food_position, 0.7)
        .hazard("bramble", Vec3f::new(4.5, 0.0, 0.0), 0.25)
        .terrain_zone(
            1,
            "meadow",
            TerrainZoneKind::Meadow,
            Vec3f::ZERO,
            3.0,
            0.8,
            0.0,
        )
        .terrain_zone(
            2,
            "ash-field",
            TerrainZoneKind::HazardField,
            hazard_position,
            2.0,
            0.1,
            0.65,
        )
        .track_resource("berry", 1, 2, 4)
        .resource_spawn_policy("seed-berry", 1, 2, 2, 0.35)
        .build()?;
    let food = world
        .entity_id("berry")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G07 scenario must include tracked food",
        })?;

    let mut mind = CreatureMind::scaffold(organism_id, BrainScaleTier::Nano512, SEED, Tick::ZERO)?;
    {
        let homeostasis = mind.homeostasis_mut();
        homeostasis.drives.hunger = 0.78;
        homeostasis.drives.fatigue = 0.38;
        homeostasis.drives.fear = 0.04;
        homeostasis.drives.brain_atp = 0.58;
        homeostasis.validate_contract()?;
    }

    let mut live = LiveBrainLoop::new(world, mind, organism_id, true);
    let mut tick_summaries = Vec::new();
    let mut hazard_pain = 0.0;
    let mut hazard_tick = Tick::ZERO;
    let scripted = [
        proposal(
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            Some(food),
            None,
            0.96,
            0.97,
            0.8,
        )?,
        proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.84,
            0.9,
            0.0,
        )?,
        proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.83,
            0.9,
            0.0,
        )?,
        proposal(
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            None,
            Some(hazard_position),
            0.93,
            0.95,
            4.0,
        )?,
    ];

    for action in scripted {
        let before_pain = live.mind().homeostasis().drives.pain;
        let summary = live.tick_with_proposals(vec![action]);
        if live.mind().homeostasis().drives.pain > before_pain {
            hazard_tick = summary.tick_after;
            hazard_pain = live.mind().homeostasis().drives.pain - before_pain;
        }
        tick_summaries.push(summary);
    }

    let indicators = live.ecology_indicators();
    let metrics = live.ecology_metrics();
    let (sealed_patch_count, packed_record_count) = live.telemetry_counts();
    let summary = PlayableEcologyLoopSummary {
        schema: G07_WORLD_ECOLOGY_SCHEMA,
        schema_version: G07_WORLD_ECOLOGY_SCHEMA_VERSION,
        seed: SEED,
        organism_id,
        tick_summaries,
        ecology_indicators: indicators,
        metrics,
        regrown_resource_id: Some(food),
        spawned_labels: live
            .world_signature()
            .into_iter()
            .filter(|line| line.contains("seed-berry"))
            .collect(),
        hazard_tick,
        hazard_pain: hazard_pain.clamp(0.0, 1.0),
        sensory_zone_label: live.current_ecology_zone_label()?,
        world_signature: live.world_signature(),
        sealed_patch_count,
        packed_record_count,
    };
    summary.validate()?;
    Ok(summary)
}
