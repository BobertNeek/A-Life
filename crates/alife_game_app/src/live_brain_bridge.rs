//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainRunMode {
    Paused,
    StepOnce,
    RunFixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveBrainTickControl {
    pub mode: LiveBrainRunMode,
    pub fixed_ticks: u32,
}

impl LiveBrainTickControl {
    pub const fn paused() -> Self {
        Self {
            mode: LiveBrainRunMode::Paused,
            fixed_ticks: 0,
        }
    }

    pub const fn step_once() -> Self {
        Self {
            mode: LiveBrainRunMode::StepOnce,
            fixed_ticks: 1,
        }
    }

    pub const fn run_fixed(fixed_ticks: u32) -> Self {
        Self {
            mode: LiveBrainRunMode::RunFixed,
            fixed_ticks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainCausalStage {
    GatherSensory,
    CpuBrainTick,
    ExecuteAction,
    MeasureOutcome,
    SealPatch,
    UpdateLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveBrainTickSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub world_tick_before: Tick,
    pub world_tick_after: Tick,
    pub status: BrainTickStatus,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub patch_sealed: bool,
    pub patch_sequence_id: Option<u64>,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_updates: u32,
    pub topology_updates: u32,
    pub learning_updates: u32,
    pub causal_stages: Vec<LiveBrainCausalStage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveBrainTickDetailed {
    pub summary: LiveBrainTickSummary,
    pub sealed_patch: Option<ExperiencePatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveBrainTickBatch {
    pub summaries: Vec<LiveBrainTickSummary>,
    pub motor_ring: Option<MotorRingPresentation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveBrainProposalScores {
    pub food_score: f32,
    pub hazard_score: f32,
    pub inspect_score: f32,
    pub idle_score: f32,
    pub confidence: f32,
}

#[derive(Debug)]
pub struct LiveBrainLoop {
    organism_id: OrganismId,
    logging_enabled: bool,
    harness: HeadlessBrainHarness,
    mind: CreatureMind,
}

impl LiveBrainLoop {
    pub fn from_p34_launch(launch: &AppShellLaunchConfig) -> Result<Self, GameAppShellError> {
        let config = RuntimeConfig::from_json_file(&launch.config_path)?;
        config.validate()?;
        let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
        manifest.validate_with_root(&launch.asset_root)?;
        let save = PortableSaveFile::from_json_file(&launch.save_path)?;
        save.validate_with_asset_root(&launch.asset_root)?;
        if save.deterministic_seed != config.deterministic_seed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "runtime config seed must match portable save seed",
            });
        }
        let creature = save
            .creatures
            .first()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "portable save must include at least one creature for G03",
            })?;
        let world = save.restore_headless_world()?;
        let mut mind = CreatureMind::scaffold(
            creature.organism_id,
            creature.brain_class,
            save.deterministic_seed,
            creature.mind.tick,
        )?;
        *mind.homeostasis_mut() = creature.mind.homeostasis;
        mind.homeostasis().validate_contract()?;
        Ok(Self::new(
            world,
            mind,
            creature.organism_id,
            config.logging.enabled,
        ))
    }

    pub fn new(
        world: HeadlessWorld,
        mind: CreatureMind,
        organism_id: OrganismId,
        logging_enabled: bool,
    ) -> Self {
        Self {
            organism_id,
            logging_enabled,
            harness: HeadlessBrainHarness::new(world),
            mind,
        }
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn mind(&self) -> &CreatureMind {
        &self.mind
    }

    pub fn initialize_neural_projection_schema(
        &mut self,
        schema: NeuralProjectionSchema,
    ) -> Result<(), GameAppShellError> {
        self.mind.initialize_neural_projection_schema(schema)?;
        Ok(())
    }

    pub fn apply_post_seal_lifetime_deltas(
        &mut self,
        sealed_patch: &ExperiencePatch,
        deltas: PostSealLifetimeDeltaBatch,
    ) -> Result<PostSealLifetimeDeltaReceipt, GameAppShellError> {
        Ok(self
            .mind
            .apply_post_seal_lifetime_deltas(sealed_patch, deltas)?)
    }

    pub fn creature_visual_snapshot(
        &self,
        presentation: &VisibleWorldPresentation,
        last_tick: Option<&LiveBrainTickSummary>,
    ) -> Result<CreatureVisualSnapshot, GameAppShellError> {
        creature_visual_snapshot_from_presentation(
            presentation,
            self.organism_id,
            &self.mind,
            last_tick,
        )
    }

    pub fn world_signature(&self) -> Vec<String> {
        self.harness.world().stable_signature()
    }

    pub fn ecology_metrics(&self) -> EcologyMetrics {
        self.harness.world().ecology_metrics()
    }

    pub fn ecology_indicators(&self) -> Vec<EcologyIndicator> {
        self.harness
            .world()
            .ecology()
            .zones
            .iter()
            .map(|zone| EcologyIndicator {
                zone_id: zone.id,
                label: zone.label.clone(),
                terrain_kind: zone.kind,
                resource_bias: zone.resource_bias,
                hazard_pressure: zone.hazard_pressure,
            })
            .collect()
    }

    pub fn current_ecology_zone_label(&self) -> Result<Option<String>, GameAppShellError> {
        let report = self
            .harness
            .world()
            .sensory_report(self.organism_id, self.mind.current_tick())?;
        Ok(report.ecology.current_zone.and_then(|zone_id| {
            self.harness
                .world()
                .ecology()
                .zones
                .iter()
                .find(|zone| zone.id == zone_id)
                .map(|zone| zone.label.clone())
        }))
    }

    pub fn telemetry_counts(&self) -> (usize, usize) {
        (
            self.harness.telemetry().sealed_patches.len(),
            self.harness.telemetry().packed_records.len(),
        )
    }

    pub fn update(
        &mut self,
        control: LiveBrainTickControl,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        Ok(self.update_with_motor_ring(control)?.summaries)
    }

    pub fn update_with_motor_ring(
        &mut self,
        control: LiveBrainTickControl,
    ) -> Result<LiveBrainTickBatch, GameAppShellError> {
        let ticks = match control.mode {
            LiveBrainRunMode::Paused => 0,
            LiveBrainRunMode::StepOnce => 1,
            LiveBrainRunMode::RunFixed => control.fixed_ticks.min(16),
        };
        let mut summaries = Vec::with_capacity(ticks as usize);
        let mut motor_ring = None;
        for _ in 0..ticks {
            let proposals = self.proposals_from_current_sensory()?;
            motor_ring = Some(MotorRingPresentation::from_proposals(
                self.organism_id,
                &proposals,
            )?);
            summaries.push(self.tick_with_proposals(proposals));
        }
        Ok(LiveBrainTickBatch {
            summaries,
            motor_ring,
        })
    }

    pub fn current_context_proposals(&self) -> Result<Vec<ActionProposal>, GameAppShellError> {
        self.proposals_from_current_sensory()
    }

    pub fn current_motor_ring_presentation(
        &self,
    ) -> Result<MotorRingPresentation, GameAppShellError> {
        let proposals = self.proposals_from_current_sensory()?;
        MotorRingPresentation::from_proposals(self.organism_id, &proposals)
    }

    pub fn current_sensory_report(&self) -> Result<HeadlessSensoryReport, GameAppShellError> {
        self.harness
            .world()
            .sensory_report(self.organism_id, self.mind.current_tick())
            .map_err(GameAppShellError::from)
    }

    pub fn current_context_proposals_with_scores(
        &self,
        scores: LiveBrainProposalScores,
    ) -> Result<Vec<ActionProposal>, GameAppShellError> {
        let report = self.current_sensory_report()?;
        self.proposals_from_sensory_report(report, Some(scores))
    }

    pub fn tick_with_proposals(&mut self, proposals: Vec<ActionProposal>) -> LiveBrainTickSummary {
        self.tick_with_proposals_detailed(proposals, true).summary
    }

    pub fn tick_with_proposals_detailed(
        &mut self,
        proposals: Vec<ActionProposal>,
        enable_learning_trace_update: bool,
    ) -> LiveBrainTickDetailed {
        let tick_before = self.mind.current_tick();
        let world_tick_before = self.harness.world().tick();
        let mut input = BrainTickInput::new(tick_before, proposals)
            .with_pack_experience(self.logging_enabled)
            .with_action_duration(DurationTicks::new(1));
        input.enable_learning_trace_update = enable_learning_trace_update;
        let tick = self.harness.tick_mind(&mut self.mind, input);
        let world_tick_after = self.harness.world().tick();
        let action_failure = tick
            .action_result
            .as_ref()
            .and_then(|result| result.execution.failure);
        let summary = Self::summarize_tick(
            self.organism_id,
            tick_before,
            self.mind.current_tick(),
            world_tick_before,
            world_tick_after,
            &tick.brain,
            action_failure,
            self.harness.telemetry().sealed_patches.len(),
            self.harness.telemetry().packed_records.len(),
        );
        LiveBrainTickDetailed {
            summary,
            sealed_patch: tick.brain.experience_patch,
        }
    }

    fn proposals_from_current_sensory(&self) -> Result<Vec<ActionProposal>, GameAppShellError> {
        let report = self.current_sensory_report()?;
        self.proposals_from_sensory_report(report, None)
    }

    fn proposals_from_sensory_report(
        &self,
        report: HeadlessSensoryReport,
        scores: Option<LiveBrainProposalScores>,
    ) -> Result<Vec<ActionProposal>, GameAppShellError> {
        let mut proposals = Vec::new();
        for visible in report.visible_entities {
            match visible.kind {
                WorldObjectKind::Food => {
                    let (action_id, kind) = if visible.distance > CA16_EAT_REACH_DISTANCE {
                        (HeadlessActionIds::APPROACH, ActionKind::Move)
                    } else {
                        (HeadlessActionIds::EAT, ActionKind::Interact)
                    };
                    proposals.push(proposal(
                        action_id,
                        kind,
                        Some(visible.id),
                        None,
                        scores.map_or(0.72, |scores| scores.food_score),
                        scores.map_or(0.95, |scores| scores.confidence),
                        visible.distance,
                    )?)
                }
                WorldObjectKind::Hazard => proposals.push(proposal(
                    HeadlessActionIds::FLEE,
                    ActionKind::Move,
                    Some(visible.id),
                    None,
                    scores.map_or(0.66, |scores| scores.hazard_score),
                    scores.map_or(0.9, |scores| scores.confidence),
                    visible.distance,
                )?),
                WorldObjectKind::Obstacle => proposals.push(proposal(
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    Some(visible.id),
                    None,
                    scores.map_or(0.38, |scores| scores.inspect_score),
                    0.7,
                    visible.distance,
                )?),
                WorldObjectKind::Agent | WorldObjectKind::Token => proposals.push(proposal(
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    Some(visible.id),
                    None,
                    scores.map_or(0.42, |scores| scores.inspect_score),
                    0.75,
                    visible.distance,
                )?),
            }
        }
        proposals.push(proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            scores.map_or(0.28, |scores| scores.idle_score),
            0.55,
            0.0,
        )?);
        Ok(proposals)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn summarize_tick(
        organism_id: OrganismId,
        tick_before: Tick,
        tick_after: Tick,
        world_tick_before: Tick,
        world_tick_after: Tick,
        brain: &alife_core::BrainTickOutput,
        action_failure: Option<ReferenceActionFailure>,
        sealed_patch_count: usize,
        packed_record_count: usize,
    ) -> LiveBrainTickSummary {
        let patch = brain.experience_patch.as_ref();
        let selected = brain.selected_action;
        LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before,
            tick_after,
            world_tick_before,
            world_tick_after,
            status: brain.status,
            selected_action_kind: selected.map(|command| command.kind),
            selected_action_id: selected.map(|command| command.action_id),
            target_entity: selected.and_then(|command| command.target_entity),
            patch_sealed: patch.is_some(),
            patch_sequence_id: patch.map(|patch| patch.pre_action().sequence_id.raw()),
            patch_success: patch.map(|patch| patch.outcome().success),
            physical_contact: patch.map(|patch| patch.outcome().physical.contact),
            action_failure,
            sealed_patch_count,
            packed_record_count,
            memory_updates: brain.diagnostics.memory_updates,
            topology_updates: brain.diagnostics.topology_updates,
            learning_updates: brain.diagnostics.learning_updates,
            causal_stages: vec![
                LiveBrainCausalStage::GatherSensory,
                LiveBrainCausalStage::CpuBrainTick,
                LiveBrainCausalStage::ExecuteAction,
                LiveBrainCausalStage::MeasureOutcome,
                LiveBrainCausalStage::SealPatch,
                LiveBrainCausalStage::UpdateLogs,
            ],
        }
    }
}

pub(crate) fn proposal(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
    score: f32,
    confidence: f32,
    distance: f32,
) -> Result<ActionProposal, ScaffoldContractError> {
    let salience = if distance <= 0.0 {
        0.5
    } else {
        (1.0 / (1.0 + distance)).clamp(0.1, 1.0)
    };
    let mut proposal = ActionProposal::new(
        action_id,
        kind,
        score,
        Confidence::new(confidence)?,
        None,
        0b11,
        ActionTarget::new(target_entity, target_position),
        NormalizedScalar::new(salience)?,
    )?;
    proposal.intensity = Intensity::new(1.0)?;
    Ok(proposal)
}

pub fn run_live_brain_loop_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<LiveBrainTickSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut summaries = live.update(LiveBrainTickControl::step_once())?;
    summaries
        .pop()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "step once must produce one live brain tick",
        })
}

pub fn run_live_brain_loop_fixed_smoke(
    launch: &AppShellLaunchConfig,
    ticks: u32,
) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    live.update(LiveBrainTickControl::run_fixed(ticks))
}

pub fn run_live_brain_loop_paused_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<(Tick, Tick, usize), GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mind_tick = live.mind.current_tick();
    let world_tick = live.harness.world().tick();
    let summaries = live.update(LiveBrainTickControl::paused())?;
    Ok((mind_tick, world_tick, summaries.len()))
}
