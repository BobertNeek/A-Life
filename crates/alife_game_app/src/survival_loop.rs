//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayableSurvivalEventKind {
    FoodConsumed,
    MissingAffordance,
    HazardPain,
    RestSleep,
}

impl PlayableSurvivalEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FoodConsumed => "food-consumed",
            Self::MissingAffordance => "missing-affordance",
            Self::HazardPain => "hazard-pain",
            Self::RestSleep => "rest-sleep",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableSurvivalEvent {
    pub kind: PlayableSurvivalEventKind,
    pub tick: Tick,
    pub action_kind: Option<ActionKind>,
    pub target_entity: Option<WorldEntityId>,
    pub success: bool,
    pub contact: Option<PhysicalContactKind>,
    pub hunger_before: f32,
    pub hunger_after: f32,
    pub fatigue_after: f32,
    pub fear_after: f32,
    pub pain_after: f32,
    pub brain_atp_after: f32,
    pub sleep_phase_after: SleepPhase,
    pub message: String,
}

impl PlayableSurvivalEvent {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        for value in [
            self.hunger_before,
            self.hunger_after,
            self.fatigue_after,
            self.fear_after,
            self.pain_after,
            self.brain_atp_after,
        ] {
            NormalizedScalar::new(value)?;
        }
        if self.message.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{:?}:{}:{:?}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:?}",
            self.kind.label(),
            self.action_kind,
            self.target_entity.map(|id| id.raw()),
            self.success,
            self.contact,
            self.hunger_before,
            self.hunger_after,
            self.fatigue_after,
            self.fear_after,
            self.pain_after,
            self.sleep_phase_after
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableSurvivalLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub organism_id: OrganismId,
    pub object_count: usize,
    pub events: Vec<PlayableSurvivalEvent>,
    pub tick_summaries: Vec<LiveBrainTickSummary>,
    pub final_visual: CreatureVisualSnapshot,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub unresolved_gap_count: usize,
    pub world_signature: Vec<String>,
}

impl PlayableSurvivalLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema != G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA
            || self.schema_version != G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.events.len() != 4 || self.tick_summaries.len() != 4 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.object_count < 4
            || self.sealed_patch_count < self.events.len()
            || self.packed_record_count < self.events.len()
            || self.memory_record_count < self.events.len()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for event in &self.events {
            event.validate()?;
        }
        self.final_visual.validate()?;
        Ok(())
    }

    pub fn event_labels(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.kind.label()).collect()
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.organism_id.raw(),
            self.object_count,
            self.event_labels().join(">"),
            self.sealed_patch_count,
            self.final_visual.signature_line()
        )
    }
}

pub fn run_playable_survival_loop_smoke() -> Result<PlayableSurvivalLoopSummary, GameAppShellError>
{
    const SEED: u64 = 6_060;
    let organism_id = OrganismId(606);
    let food_position = Vec3f::new(1.0, 0.0, 0.0);
    let hazard_position = Vec3f::new(2.0, 0.0, 0.0);
    let world = HeadlessScenarioBuilder::new(SEED)
        .agent("creature", organism_id, Vec3f::ZERO)
        .food("berry", food_position, 0.75)
        .hazard("thorn", hazard_position, 0.8)
        .obstacle("stone", Vec3f::new(-1.5, 0.0, 0.0), 0.75)
        .token("rest-nest", Vec3f::new(0.0, 1.0, 0.0), 60_600)
        .build()?;
    let food = world
        .entity_id("berry")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G06 scenario must include food",
        })?;
    let hazard = world
        .entity_id("thorn")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G06 scenario must include hazard",
        })?;
    let object_count = world.stable_signature().len();
    let mut mind = CreatureMind::scaffold(organism_id, BrainScaleTier::Nano512, SEED, Tick::ZERO)?;
    {
        let homeostasis = mind.homeostasis_mut();
        homeostasis.drives.hunger = 0.82;
        homeostasis.drives.fatigue = 0.72;
        homeostasis.drives.fear = 0.05;
        homeostasis.drives.pain = 0.0;
        homeostasis.drives.brain_atp = 0.54;
        homeostasis.hormones.sleep_pressure = 0.76;
        homeostasis.validate_contract()?;
    }

    let mut live = LiveBrainLoop::new(world, mind, organism_id, true);
    let mut tick_summaries = Vec::new();
    let mut events = Vec::new();
    let scripted = [
        (
            PlayableSurvivalEventKind::FoodConsumed,
            proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.96,
                0.97,
                1.0,
            )?,
            "ate visible food; hunger drops and packed/sealed logs update",
        ),
        (
            PlayableSurvivalEventKind::MissingAffordance,
            proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.94,
                0.95,
                1.0,
            )?,
            "tried consumed food once; failure is recoverable and bounded",
        ),
        (
            PlayableSurvivalEventKind::HazardPain,
            proposal(
                ActionKind::Move.canonical_id(),
                ActionKind::Move,
                Some(hazard),
                None,
                0.93,
                0.94,
                1.0,
            )?,
            "entered visible hazard; pain/fear rise and topology gap remains bias-only",
        ),
        (
            PlayableSurvivalEventKind::RestSleep,
            proposal(
                ActionKind::Rest.canonical_id(),
                ActionKind::Rest,
                None,
                None,
                0.91,
                0.92,
                0.0,
            )?,
            "rest action succeeds; P16 forced sleep hook becomes visible",
        ),
    ];

    for (kind, action, message) in scripted {
        let before = *live.mind().homeostasis();
        let summary = live.tick_with_proposals(vec![action]);
        let after = live.mind().homeostasis();
        let event = PlayableSurvivalEvent {
            kind,
            tick: summary.tick_after,
            action_kind: summary.selected_action_kind,
            target_entity: summary.target_entity,
            success: summary.patch_success.unwrap_or(false),
            contact: summary.physical_contact,
            hunger_before: before.drives.hunger,
            hunger_after: after.drives.hunger,
            fatigue_after: after.drives.fatigue,
            fear_after: after.drives.fear,
            pain_after: after.drives.pain,
            brain_atp_after: after.drives.brain_atp,
            sleep_phase_after: live.mind().sleep_state().phase,
            message: message.to_string(),
        };
        event.validate()?;
        events.push(event);
        tick_summaries.push(summary);
    }

    let (sealed_patch_count, packed_record_count) = live.telemetry_counts();
    let final_visual = creature_visual_snapshot_from_parts(
        organism_id,
        WorldEntityId(1),
        hazard_position,
        None,
        None,
        live.mind().homeostasis(),
        live.mind().sleep_state().phase,
        tick_summaries
            .last()
            .and_then(|summary| summary.selected_action_kind),
    )?;
    let summary = PlayableSurvivalLoopSummary {
        schema: G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA,
        schema_version: G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION,
        seed: SEED,
        organism_id,
        object_count,
        events,
        tick_summaries,
        final_visual,
        sealed_patch_count,
        packed_record_count,
        memory_record_count: live.mind().memory_bank().len(),
        topology_concept_count: live.mind().topological_map().concepts().len(),
        unresolved_gap_count: live.mind().topological_map().unresolved_gaps().len(),
        world_signature: live.world_signature(),
    };
    summary.validate()?;
    Ok(summary)
}
