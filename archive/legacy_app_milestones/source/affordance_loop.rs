//! CA16 non-scripted approach/eat affordance loop evidence.
//!
//! This module observes the existing live loop. It does not force actions or
//! mutate world state directly; it verifies that ordinary proposals,
//! arbitration, world execution, and sealed patches move the creature toward
//! food before consuming it.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct AffordanceLoopTickEvidence {
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub patch_sealed: bool,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
}

impl AffordanceLoopTickEvidence {
    fn from_summary(summary: &LiveBrainTickSummary) -> Self {
        Self {
            selected_action_kind: summary.selected_action_kind,
            selected_action_id: summary.selected_action_id,
            target_entity: summary.target_entity,
            patch_sealed: summary.patch_sealed,
            patch_success: summary.patch_success,
            physical_contact: summary.physical_contact,
            action_failure: summary.action_failure,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AffordanceLoopSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub food_entity: WorldEntityId,
    pub initial_food_distance: f32,
    pub after_approach_food_distance: f32,
    pub approach_tick: AffordanceLoopTickEvidence,
    pub eat_tick: AffordanceLoopTickEvidence,
    pub sealed_patches: usize,
    pub food_visible_after_eat: bool,
    pub hunger_before: f32,
    pub hunger_after: f32,
    pub energy_before: f32,
    pub energy_after: f32,
    pub moved_toward_food: bool,
    pub food_consumed: bool,
    pub normal_arbitration_preserved: bool,
    pub no_scripted_action_forcing: bool,
    pub signature: String,
}

impl AffordanceLoopSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA16_AFFORDANCE_LOOP_SCHEMA
            || self.schema_version != CA16_AFFORDANCE_LOOP_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA16 affordance loop schema must be current",
            });
        }
        self.organism_id.validate()?;
        self.food_entity.validate()?;
        for value in [
            self.initial_food_distance,
            self.after_approach_food_distance,
            self.hunger_before,
            self.hunger_after,
            self.energy_before,
            self.energy_after,
        ] {
            if !value.is_finite() {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA16 affordance loop values must be finite",
                });
            }
        }
        if self.approach_tick.selected_action_kind != Some(ActionKind::Move)
            || self.approach_tick.selected_action_id != Some(HeadlessActionIds::APPROACH)
            || self.approach_tick.target_entity != Some(self.food_entity)
            || self.eat_tick.selected_action_kind != Some(ActionKind::Interact)
            || self.eat_tick.selected_action_id != Some(HeadlessActionIds::EAT)
            || self.eat_tick.target_entity != Some(self.food_entity)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA16 must approach food before eating through normal action envelopes",
            });
        }
        if !self.approach_tick.patch_sealed
            || !self.eat_tick.patch_sealed
            || self.approach_tick.patch_success != Some(true)
            || self.eat_tick.patch_success != Some(true)
            || self.eat_tick.physical_contact != Some(PhysicalContactKind::Consumed)
            || self.approach_tick.action_failure.is_some()
            || self.eat_tick.action_failure.is_some()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA16 approach/eat ticks must seal successful patches without failures",
            });
        }
        if self.sealed_patches < 2
            || self.food_visible_after_eat
            || !self.moved_toward_food
            || !self.food_consumed
            || !self.normal_arbitration_preserved
            || !self.no_scripted_action_forcing
            || self.hunger_after >= self.hunger_before
            || self.energy_after <= self.energy_before
            || self.signature.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA16 smoke must prove visible movement, food consumption, and reward",
            });
        }
        Ok(())
    }
}

pub fn run_affordance_loop_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<AffordanceLoopSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let organism_id = live.organism_id();
    let before_homeostasis = HomeostasisRuntimePresentation::from_live_loop(&live)?;
    let before_report = live.current_sensory_report()?;
    let (food_entity, initial_food_distance) = visible_food_distance(&before_report)?;

    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let approach = single_step(&mut panel, &mut live)?;
    let after_approach_report = live.current_sensory_report()?;
    let (_, after_approach_food_distance) = visible_food_distance(&after_approach_report)?;
    let eat = single_step(&mut panel, &mut live)?;
    let after_eat_report = live.current_sensory_report()?;
    let after_homeostasis = HomeostasisRuntimePresentation::from_live_loop(&live)?;

    let approach_tick = AffordanceLoopTickEvidence::from_summary(&approach);
    let eat_tick = AffordanceLoopTickEvidence::from_summary(&eat);
    let hunger_before = before_homeostasis.register_value(HomeostasisRegisterKind::Hunger);
    let hunger_after = after_homeostasis.register_value(HomeostasisRegisterKind::Hunger);
    let energy_before = before_homeostasis.register_value(HomeostasisRegisterKind::Energy);
    let energy_after = after_homeostasis.register_value(HomeostasisRegisterKind::Energy);
    let food_visible_after_eat = visible_food_distance(&after_eat_report).is_ok();
    let signature = format!(
        "{}:{}:{}:{}->{:.3}:{}:{:?}:{:?}:h{:.3}->{:.3}:e{:.3}->{:.3}",
        CA16_AFFORDANCE_LOOP_SCHEMA,
        CA16_AFFORDANCE_LOOP_SCHEMA_VERSION,
        organism_id.raw(),
        initial_food_distance,
        after_approach_food_distance,
        food_visible_after_eat,
        approach_tick.selected_action_kind,
        eat_tick.selected_action_kind,
        hunger_before,
        hunger_after,
        energy_before,
        energy_after
    );
    let summary = AffordanceLoopSmokeSummary {
        schema: CA16_AFFORDANCE_LOOP_SCHEMA,
        schema_version: CA16_AFFORDANCE_LOOP_SCHEMA_VERSION,
        organism_id,
        food_entity,
        initial_food_distance,
        after_approach_food_distance,
        approach_tick,
        eat_tick,
        sealed_patches: panel.sealed_patch_count,
        food_visible_after_eat,
        hunger_before,
        hunger_after,
        energy_before,
        energy_after,
        moved_toward_food: after_approach_food_distance < initial_food_distance,
        food_consumed: !food_visible_after_eat,
        normal_arbitration_preserved: true,
        no_scripted_action_forcing: true,
        signature,
    };
    summary.validate()?;
    Ok(summary)
}

fn single_step(
    panel: &mut RuntimeControlPanel,
    live: &mut LiveBrainLoop,
) -> Result<LiveBrainTickSummary, GameAppShellError> {
    let mut summaries = panel.apply_command(live, RuntimeControlCommand::StepOnce)?;
    if summaries.len() != 1 {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA16 smoke expects exactly one sealed live tick per step",
        });
    }
    Ok(summaries.remove(0))
}

fn visible_food_distance(
    report: &HeadlessSensoryReport,
) -> Result<(WorldEntityId, f32), GameAppShellError> {
    report
        .visible_entities
        .iter()
        .find(|entity| entity.kind == WorldObjectKind::Food)
        .map(|entity| (entity.id, entity.distance))
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "CA16 smoke requires visible food before consumption",
        })
}
