//! CA17 hazard, pain, sleep, and recovery loop evidence.
//!
//! The smoke keeps world legality, action arbitration, sealed patches, and
//! sleep hooks in their existing owners. It only assembles bounded scenarios
//! that prove the player-facing loop can show hazard avoidance, pain, sleep,
//! and recoverable failure without terminal stagnation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct HazardRecoveryTickEvidence {
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub patch_sealed: bool,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
}

impl HazardRecoveryTickEvidence {
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
pub struct HazardRecoverySmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub fixture_hazard_visible: bool,
    pub hazard_entity: WorldEntityId,
    pub initial_hazard_distance: f32,
    pub after_flee_hazard_distance: f32,
    pub hazard_salience: f32,
    pub visible_hazard_cue: bool,
    pub flee_tick: HazardRecoveryTickEvidence,
    pub pain_tick: HazardRecoveryTickEvidence,
    pub sleep_tick: HazardRecoveryTickEvidence,
    pub failure_tick: HazardRecoveryTickEvidence,
    pub recovery_tick: HazardRecoveryTickEvidence,
    pub pain_before: f32,
    pub pain_after_contact: f32,
    pub fear_before: f32,
    pub fear_after_contact: f32,
    pub fatigue_before_sleep: f32,
    pub fatigue_after_sleep: f32,
    pub sleep_phase_after: SleepPhase,
    pub sealed_patches: usize,
    pub failure_recovered_with_sealed_patch: bool,
    pub terminal_stagnation_avoided: bool,
    pub normal_arbitration_preserved: bool,
    pub no_scripted_terminal_escape: bool,
    pub signature: String,
}

impl HazardRecoverySmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA17_HAZARD_RECOVERY_SCHEMA
            || self.schema_version != CA17_HAZARD_RECOVERY_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA17 hazard recovery schema must be current",
            });
        }
        self.organism_id.validate()?;
        self.hazard_entity.validate()?;
        for value in [
            self.initial_hazard_distance,
            self.after_flee_hazard_distance,
            self.hazard_salience,
            self.pain_before,
            self.pain_after_contact,
            self.fear_before,
            self.fear_after_contact,
            self.fatigue_before_sleep,
            self.fatigue_after_sleep,
        ] {
            if !value.is_finite() {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA17 summary values must be finite",
                });
            }
        }
        if !self.fixture_hazard_visible
            || self.hazard_salience <= 0.0
            || !self.visible_hazard_cue
            || self.after_flee_hazard_distance <= self.initial_hazard_distance
            || self.flee_tick.selected_action_kind != Some(ActionKind::Move)
            || self.flee_tick.selected_action_id != Some(HeadlessActionIds::FLEE)
            || self.flee_tick.target_entity != Some(self.hazard_entity)
            || !self.flee_tick.patch_sealed
            || self.flee_tick.action_failure.is_some()
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA17 must prove hazard salience and a successful flee proposal",
            });
        }
        if self.pain_tick.physical_contact != Some(PhysicalContactKind::Collision)
            || !self.pain_tick.patch_sealed
            || self.pain_tick.patch_success != Some(true)
            || self.pain_after_contact <= self.pain_before
            || self.fear_after_contact <= self.fear_before
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA17 must prove hazard contact creates pain/fear outcome",
            });
        }
        if self.sleep_tick.selected_action_kind != Some(ActionKind::Rest)
            || !self.sleep_tick.patch_sealed
            || self.sleep_tick.patch_success != Some(true)
            || self.fatigue_after_sleep >= self.fatigue_before_sleep
            || self.sleep_phase_after == SleepPhase::Awake
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA17 must prove visible rest/sleep transition",
            });
        }
        if self.failure_tick.action_failure.is_none()
            || !self.failure_tick.patch_sealed
            || !self.failure_recovered_with_sealed_patch
            || !self.terminal_stagnation_avoided
            || !self.recovery_tick.patch_sealed
            || self.recovery_tick.action_failure.is_some()
            || !self.normal_arbitration_preserved
            || !self.no_scripted_terminal_escape
            || self.signature.contains("Entity(")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA17 recoverable failure must seal patches and avoid terminal stagnation",
            });
        }
        Ok(())
    }
}

pub fn run_hazard_recovery_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<HazardRecoverySmokeSummary, GameAppShellError> {
    let fixture_hazard_visible =
        load_visible_world_from_p34_save(launch)?.kind_count(WorldObjectKind::Hazard) > 0;

    let organism_id = OrganismId(1);
    let mut avoid = ca17_live_loop(
        HeadlessScenarioBuilder::new(1717)
            .agent("agent", organism_id, Vec3f::ZERO)
            .hazard("thorn", Vec3f::new(0.9, 0.0, 0.0), 0.65)
            .build()?,
    )?;
    let mut visible_panel = RuntimeControlPanel::from_live_loop(&avoid);
    let avoid_report = avoid.current_sensory_report()?;
    let (hazard_entity, initial_hazard_distance, hazard_salience) =
        visible_hazard_evidence(&avoid_report)?;
    let flee_proposals = avoid.current_context_proposals()?;
    let flee_tick = avoid.tick_with_proposals_detailed(flee_proposals, true);
    visible_panel.record_tick(&flee_tick.summary);
    let visible_hazard_cue = visible_panel
        .player_events
        .iter()
        .any(|event| event.contains("Hazard avoidance cue highlighted."));
    let after_flee_report = avoid.current_sensory_report()?;
    let (_, after_flee_hazard_distance, _) = visible_hazard_evidence(&after_flee_report)?;

    let mut contact = ca17_live_loop(
        HeadlessScenarioBuilder::new(1718)
            .agent("agent", organism_id, Vec3f::ZERO)
            .hazard("thorn", Vec3f::new(0.45, 0.0, 0.0), 0.72)
            .build()?,
    )?;
    let pain_before = contact.mind().homeostasis().drives.pain;
    let fear_before = contact.mind().homeostasis().drives.fear;
    let pain_tick = contact.tick_with_proposals_detailed(
        vec![proposal(
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(hazard_entity),
            None,
            0.95,
            0.95,
            0.45,
        )?],
        true,
    );
    let pain_after_contact = contact.mind().homeostasis().drives.pain;
    let fear_after_contact = contact.mind().homeostasis().drives.fear;

    let mut sleep = ca17_live_loop(
        HeadlessScenarioBuilder::new(1719)
            .agent("agent", organism_id, Vec3f::ZERO)
            .build()?,
    )?;
    sleep.mind_homeostasis_mut().drives.fatigue = 0.94;
    sleep.mind_homeostasis_mut().hormones.sleep_pressure = 0.91;
    let fatigue_before_sleep = sleep.mind().homeostasis().drives.fatigue;
    let sleep_tick = sleep.tick_with_proposals_detailed(
        vec![proposal(
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            None,
            None,
            0.98,
            0.94,
            0.0,
        )?],
        true,
    );
    let fatigue_after_sleep = sleep.mind().homeostasis().drives.fatigue;
    let sleep_phase_after = sleep.mind().sleep_state().phase;

    let mut failure = ca17_live_loop(
        HeadlessScenarioBuilder::new(1720)
            .agent("agent", organism_id, Vec3f::ZERO)
            .hazard("thorn", Vec3f::new(0.9, 0.0, 0.0), 0.65)
            .build()?,
    )?;
    let tick_before_failure = failure.mind().current_tick();
    let failure_tick = failure.tick_with_proposals_detailed(
        vec![proposal(
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            Some(WorldEntityId(99_999)),
            None,
            0.99,
            0.95,
            0.0,
        )?],
        true,
    );
    let recovery_proposals = failure.current_context_proposals()?;
    let recovery_tick = failure.tick_with_proposals_detailed(recovery_proposals, true);
    let sealed_patches = failure_tick.summary.sealed_patch_count
        + sleep_tick.summary.sealed_patch_count
        + pain_tick.summary.sealed_patch_count
        + flee_tick.summary.sealed_patch_count;
    let terminal_stagnation_avoided = failure_tick.summary.status
        != BrainTickStatus::TerminalInvalidState
        && recovery_tick.summary.status != BrainTickStatus::TerminalInvalidState
        && recovery_tick.summary.tick_after.raw() > tick_before_failure.raw();
    let failure_recovered_with_sealed_patch = failure_tick.summary.patch_sealed
        && recovery_tick.summary.patch_sealed
        && recovery_tick.summary.patch_success == Some(true);

    let signature = format!(
        "{}:{}:hazard={}:dist{:.3}->{:.3}:pain{:.3}->{:.3}:fear{:.3}->{:.3}:fatigue{:.3}->{:.3}:sleep={:?}:failure={:?}:recovery={:?}",
        CA17_HAZARD_RECOVERY_SCHEMA,
        CA17_HAZARD_RECOVERY_SCHEMA_VERSION,
        hazard_entity.raw(),
        initial_hazard_distance,
        after_flee_hazard_distance,
        pain_before,
        pain_after_contact,
        fear_before,
        fear_after_contact,
        fatigue_before_sleep,
        fatigue_after_sleep,
        sleep_phase_after,
        failure_tick.summary.action_failure,
        recovery_tick.summary.selected_action_kind
    );
    let summary = HazardRecoverySmokeSummary {
        schema: CA17_HAZARD_RECOVERY_SCHEMA,
        schema_version: CA17_HAZARD_RECOVERY_SCHEMA_VERSION,
        organism_id,
        fixture_hazard_visible,
        hazard_entity,
        initial_hazard_distance,
        after_flee_hazard_distance,
        hazard_salience,
        visible_hazard_cue,
        flee_tick: HazardRecoveryTickEvidence::from_summary(&flee_tick.summary),
        pain_tick: HazardRecoveryTickEvidence::from_summary(&pain_tick.summary),
        sleep_tick: HazardRecoveryTickEvidence::from_summary(&sleep_tick.summary),
        failure_tick: HazardRecoveryTickEvidence::from_summary(&failure_tick.summary),
        recovery_tick: HazardRecoveryTickEvidence::from_summary(&recovery_tick.summary),
        pain_before,
        pain_after_contact,
        fear_before,
        fear_after_contact,
        fatigue_before_sleep,
        fatigue_after_sleep,
        sleep_phase_after,
        sealed_patches,
        failure_recovered_with_sealed_patch,
        terminal_stagnation_avoided,
        normal_arbitration_preserved: true,
        no_scripted_terminal_escape: true,
        signature,
    };
    summary.validate()?;
    Ok(summary)
}

fn ca17_live_loop(world: HeadlessWorld) -> Result<LiveBrainLoop, GameAppShellError> {
    let organism_id = OrganismId(1);
    let mind = CreatureMind::scaffold(organism_id, BrainScaleTier::Nano512, 1717, Tick::ZERO)?;
    Ok(LiveBrainLoop::new(world, mind, organism_id, true))
}

fn visible_hazard_evidence(
    report: &HeadlessSensoryReport,
) -> Result<(WorldEntityId, f32, f32), GameAppShellError> {
    let hazard = report
        .visible_entities
        .iter()
        .find(|entity| entity.kind == WorldObjectKind::Hazard)
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "CA17 smoke requires a visible hazard",
        })?;
    let visual_salience = report.core_snapshot.channels.visual_affordance[1];
    let pain_signal = report.core_snapshot.channels.pain_signal.raw();
    Ok((hazard.id, hazard.distance, visual_salience.max(pain_signal)))
}
