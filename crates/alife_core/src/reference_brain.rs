//! Temporary explicit `HeuristicBaseline` comparison runtime in pure Rust.
//!
//! This separately labelled baseline never shadows, gates, or replaces the
//! GPU-authoritative production neural policy. It orchestrates existing core
//! contracts and owns no world simulation, adapters, rendering, or GPU runtime.

use serde::{Deserialize, Serialize};

use crate::LifetimeTraitLedger;
use crate::{
    cpu_reference_arbitrate, cpu_spmv_projection, finalize_cpu_activations,
    update_oja_shadow_traces, validate_finite_slice, ActionArbitrationConfig,
    ActionArbitrationTraceRef, ActionBiasSource, ActionCandidate, ActionCommand,
    ActionDecisionStatus, ActionKind, ActionProposal, ActionScoreBias, BodySnapshot,
    BrainClassSpec, BrainGenome, BrainScaleTier, CandidateActionFamily, CandidateFeatureVector,
    CandidateObservationRef, ChemistryModulation, Confidence, ContractDiagnostic, CpuNeuralState,
    DecisionSnapshot, DevelopmentState, DurationTicks, ExperiencePacker, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HomeostaticDelta, HomeostaticParameters,
    HomeostaticSnapshot, Intensity, LobeKind, MemoryBank, MemoryBankConfig,
    MemoryExpectancySnapshot, MemoryId, NeuralActivationConfig, NeuralDiagnostics,
    NeuralProjectionSchema, NeuralUpdateReport, NormalizedScalar, OjaUpdateConfig, OrganismId,
    PackedExperienceRecord, PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind, Pose,
    PostActionOutcome, PreActionSnapshot, ScaffoldContractError, SensorProfile, SensorySnapshot,
    SignedValence, SleepConsolidationConfig, SleepConsolidationReport, SleepConsolidator,
    SleepController, SleepPhase, SleepState, SleepTransition, SleepTrigger, StructuralEditBatch,
    Tick, TopologicalMap, TopologicalMapConfig, TopologySidecar, TopologyUpdate, Validate, Vec3f,
    Velocity, WeightSplitContract,
};

const DEFAULT_MEMORY_CAPACITY: usize = 64;
const DEFAULT_MEMORY_FEATURES: usize = 16;
const DEFAULT_MEMORY_MATCHES: usize = 4;
const DEFAULT_MEMORY_MIN_SCORE: f32 = 0.01;
const RECENT_FAILURE_COOLDOWN_TICKS: u32 = 8;
const RECENT_FAILURE_SCORE_PENALTY: f32 = -0.35;
const DEFAULT_MIN_ACTION_SCORE: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CreatureBodyState {
    pub pose: Pose,
    pub velocity: Velocity,
}

impl CreatureBodyState {
    pub const fn at_origin() -> Self {
        Self {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        }
    }
}

impl Validate for CreatureBodyState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.pose.validate()?;
        self.velocity.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureActionState {
    pub last_failed_action_id: Option<crate::ActionId>,
    pub last_failed_tick: Option<Tick>,
    pub failure_cooldown_ticks: u32,
}

impl CreatureActionState {
    pub const fn reference() -> Self {
        Self {
            last_failed_action_id: None,
            last_failed_tick: None,
            failure_cooldown_ticks: RECENT_FAILURE_COOLDOWN_TICKS,
        }
    }

    fn recent_penalty_for(self, proposal: &ActionProposal, tick: Tick) -> f32 {
        let Some(action_id) = self.last_failed_action_id else {
            return 0.0;
        };
        let Some(last_tick) = self.last_failed_tick else {
            return 0.0;
        };
        if action_id != proposal.action_id {
            return 0.0;
        }
        let elapsed = tick.raw().saturating_sub(last_tick.raw());
        if elapsed <= u64::from(self.failure_cooldown_ticks) {
            RECENT_FAILURE_SCORE_PENALTY
        } else {
            0.0
        }
    }

    fn record_execution(&mut self, command: ActionCommand, tick: Tick, succeeded: bool) {
        if succeeded {
            if let Some(last_tick) = self.last_failed_tick {
                if tick.raw().saturating_sub(last_tick.raw())
                    > u64::from(self.failure_cooldown_ticks)
                {
                    self.last_failed_action_id = None;
                    self.last_failed_tick = None;
                }
            }
            return;
        }
        self.last_failed_action_id = Some(command.action_id);
        self.last_failed_tick = Some(tick);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainTickStatus {
    Normal,
    RecoverableActionFailure,
    TerminalInvalidState,
    SafeIdle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainTickDiagnostics {
    pub active_synapses: u32,
    pub active_tiles: u32,
    pub supertiles_skipped: u32,
    pub cpu_patch_allocations: u32,
    pub packed_log_records: u32,
    pub invalid_or_rejected_action_count: u32,
    pub recoverable_action_failures: u32,
    pub nan_or_range_rejections: u32,
    pub memory_updates: u32,
    pub topology_updates: u32,
    pub learning_updates: u32,
    pub last_diagnostic: Option<ContractDiagnostic>,
}

impl BrainTickDiagnostics {
    fn observe_neural(&mut self, report: NeuralUpdateReport) {
        self.active_synapses = self.active_synapses.saturating_add(report.active_synapses);
        self.active_tiles = self.active_tiles.saturating_add(report.active_tiles);
        self.supertiles_skipped = self
            .supertiles_skipped
            .saturating_add(report.mask_skipped_tiles);
        self.nan_or_range_rejections = self
            .nan_or_range_rejections
            .saturating_add(report.nan_rejections)
            .saturating_add(report.range_rejections);
    }

    fn observe_error(&mut self, error: &ScaffoldContractError) {
        self.last_diagnostic = Some(ContractDiagnostic::from(error));
        if matches!(
            error,
            ScaffoldContractError::NonFiniteFloat
                | ScaffoldContractError::ScalarOutOfRange
                | ScaffoldContractError::OutOfRangeDriveHormone
        ) {
            self.nan_or_range_rejections = self.nan_or_range_rejections.saturating_add(1);
        }
        if matches!(error, ScaffoldContractError::InvalidActionDecision) {
            self.invalid_or_rejected_action_count =
                self.invalid_or_rejected_action_count.saturating_add(1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainTickInput {
    pub tick: Tick,
    pub proposals: Vec<ActionProposal>,
    pub min_action_score: f32,
    pub min_action_confidence: Confidence,
    pub fallback_kind: ActionKind,
    pub action_duration: DurationTicks,
    pub pack_experience: bool,
    pub enable_learning_trace_update: bool,
}

impl BrainTickInput {
    pub fn new(tick: Tick, proposals: Vec<ActionProposal>) -> Self {
        Self {
            tick,
            proposals,
            min_action_score: DEFAULT_MIN_ACTION_SCORE,
            min_action_confidence: Confidence(0.01),
            fallback_kind: ActionKind::Inspect,
            action_duration: DurationTicks(1),
            pack_experience: false,
            enable_learning_trace_update: true,
        }
    }

    pub const fn with_pack_experience(mut self, pack_experience: bool) -> Self {
        self.pack_experience = pack_experience;
        self
    }

    pub const fn with_action_duration(mut self, action_duration: DurationTicks) -> Self {
        self.action_duration = action_duration;
        self
    }

    pub fn with_min_action_score(mut self, min_action_score: f32) -> Self {
        self.min_action_score = min_action_score;
        self
    }

    fn validate_contract(&self, expected_tick: Tick) -> Result<(), ScaffoldContractError> {
        Tick::validate_monotonic(expected_tick, self.tick)?;
        if self.tick != expected_tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        crate::validate_finite(self.min_action_score)?;
        Confidence::new(self.min_action_confidence.raw())?;
        if self.action_duration.raw() == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for proposal in &self.proposals {
            proposal.action_id.validate()?;
            proposal.target.validate()?;
            crate::validate_finite(proposal.score)?;
            Confidence::new(proposal.confidence.raw())?;
            NormalizedScalar::new(proposal.salience.raw())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainTickOutput {
    pub status: BrainTickStatus,
    pub selected_action: Option<ActionCommand>,
    pub experience_patch: Option<ExperiencePatch>,
    pub packed_record: Option<PackedExperienceRecord>,
    pub memory_update: Option<MemoryId>,
    pub topology_update: Option<TopologyUpdate>,
    pub endocrine_update: Option<HomeostaticSnapshot>,
    pub neural_report: NeuralUpdateReport,
    pub diagnostics: BrainTickDiagnostics,
}

impl BrainTickOutput {
    fn terminal(error: ScaffoldContractError, diagnostics: BrainTickDiagnostics) -> Self {
        let mut diagnostics = diagnostics;
        diagnostics.observe_error(&error);
        Self {
            status: BrainTickStatus::TerminalInvalidState,
            selected_action: None,
            experience_patch: None,
            packed_record: None,
            memory_update: None,
            topology_update: None,
            endocrine_update: None,
            neural_report: NeuralUpdateReport::default(),
            diagnostics,
        }
    }

    fn sleep_idle(diagnostics: BrainTickDiagnostics) -> Self {
        Self {
            status: BrainTickStatus::SafeIdle,
            selected_action: None,
            experience_patch: None,
            packed_record: None,
            memory_update: None,
            topology_update: None,
            endocrine_update: None,
            neural_report: NeuralUpdateReport::default(),
            diagnostics,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReferenceSensoryRequest {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub body_pose: Pose,
    pub body_velocity: Velocity,
    pub homeostasis: HomeostaticSnapshot,
}

pub trait ReferenceSensoryAdapter {
    fn gather_sensory(
        &mut self,
        request: ReferenceSensoryRequest,
    ) -> Result<SensorySnapshot, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceActionFailure {
    MissingAffordance,
    ActionRejected,
    Blocked,
    ExecutorInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReferenceActionExecution {
    pub succeeded: bool,
    pub failure: Option<ReferenceActionFailure>,
    pub physical: PhysicalActionOutcome,
}

impl ReferenceActionExecution {
    pub fn succeeded(physical: PhysicalActionOutcome) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            succeeded: true,
            failure: None,
            physical,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn failed(
        failure: ReferenceActionFailure,
        physical: PhysicalActionOutcome,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            succeeded: false,
            failure: Some(failure),
            physical,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for ReferenceActionExecution {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.physical.validate_contract()?;
        match (self.succeeded, self.failure) {
            (true, None) | (false, Some(_)) => Ok(()),
            _ => Err(ScaffoldContractError::InvalidActionDecision),
        }
    }
}

pub trait ReferenceActionExecutor {
    fn execute_action(
        &mut self,
        command: &ActionCommand,
    ) -> Result<ReferenceActionExecution, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceOutcomeRequest<'a> {
    pub command: &'a ActionCommand,
    pub execution: &'a ReferenceActionExecution,
    pub pre_action: &'a PreActionSnapshot,
    pub decision: &'a DecisionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceOutcomeObservation {
    pub success: bool,
    pub homeostatic_delta: HomeostaticDelta,
    pub reward_valence: SignedValence,
    pub frustration_delta: NormalizedScalar,
    pub pain_delta: NormalizedScalar,
    pub energy_delta: SignedValence,
    pub prediction_error: NormalizedScalar,
    pub contradiction_observed: bool,
}

impl ReferenceOutcomeObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        success: bool,
        homeostatic_delta: HomeostaticDelta,
        reward_valence: SignedValence,
        frustration_delta: NormalizedScalar,
        pain_delta: NormalizedScalar,
        energy_delta: SignedValence,
        prediction_error: NormalizedScalar,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            success,
            homeostatic_delta,
            reward_valence,
            frustration_delta,
            pain_delta,
            energy_delta,
            prediction_error,
            contradiction_observed: false,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for ReferenceOutcomeObservation {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.homeostatic_delta.validate_contract()?;
        SignedValence::new(self.reward_valence.raw())?;
        NormalizedScalar::new(self.frustration_delta.raw())?;
        NormalizedScalar::new(self.pain_delta.raw())?;
        SignedValence::new(self.energy_delta.raw())?;
        NormalizedScalar::new(self.prediction_error.raw())?;
        Ok(())
    }
}

pub trait ReferenceOutcomeObserver {
    fn observe_outcome(
        &mut self,
        request: ReferenceOutcomeRequest<'_>,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureMind {
    organism_id: OrganismId,
    brain_class: BrainClassSpec,
    genome: BrainGenome,
    development_state: DevelopmentState,
    body: CreatureBodyState,
    homeostasis: HomeostaticSnapshot,
    homeostatic_parameters: HomeostaticParameters,
    neural_state: CpuNeuralState,
    neural_schema: NeuralProjectionSchema,
    memory_bank: MemoryBank,
    topology_sidecar: TopologySidecar,
    action_state: CreatureActionState,
    sleep_controller: SleepController,
    lifetime_traits: LifetimeTraitLedger,
    pending_structural_edits: Vec<StructuralEditBatch>,
    tick: Tick,
    next_sequence_id: u64,
    deterministic_seed: u64,
    diagnostics: BrainTickDiagnostics,
}

impl CreatureMind {
    pub fn scaffold(
        organism_id: OrganismId,
        tier: BrainScaleTier,
        deterministic_seed: u64,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        let brain_class = BrainClassSpec::try_for_tier(tier)?;
        let genome = BrainGenome::scaffold(deterministic_seed, brain_class.id);
        let development_state =
            DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35)?)
                .with_enabled_lobes([
                    LobeKind::SensoryGrounding,
                    LobeKind::MetabolicDrive,
                    LobeKind::CoreAssociation,
                    LobeKind::EpisodicMemory,
                    LobeKind::MotorArbitration,
                    LobeKind::HomeostaticRegulation,
                ]);
        let mut neural_state = CpuNeuralState::for_brain_class(&brain_class)?;
        let neural_schema = NeuralProjectionSchema::empty_for_brain_class(&brain_class)?;
        neural_state.projections = neural_schema.projections.clone();
        let memory_config = MemoryBankConfig::new(
            DEFAULT_MEMORY_CAPACITY,
            DEFAULT_MEMORY_FEATURES,
            DEFAULT_MEMORY_MATCHES,
            DEFAULT_MEMORY_MIN_SCORE,
            Confidence::new(0.05)?,
        )?;
        let mind = Self {
            organism_id,
            brain_class,
            genome,
            development_state,
            body: CreatureBodyState::at_origin(),
            homeostasis: HomeostaticSnapshot::baseline(tick),
            homeostatic_parameters: HomeostaticParameters::reference(),
            neural_state,
            neural_schema,
            memory_bank: MemoryBank::new(memory_config)?,
            topology_sidecar: TopologySidecar::new(organism_id, TopologicalMapConfig::default())?,
            action_state: CreatureActionState::reference(),
            sleep_controller: SleepController::new(SleepConsolidationConfig::reference())?,
            lifetime_traits: LifetimeTraitLedger::new(64)?,
            pending_structural_edits: Vec::new(),
            tick,
            next_sequence_id: 1,
            deterministic_seed,
            diagnostics: BrainTickDiagnostics::default(),
        };
        mind.validate_ready(tick)?;
        Ok(mind)
    }

    pub const fn current_tick(&self) -> Tick {
        self.tick
    }

    pub const fn brain_class(&self) -> &BrainClassSpec {
        &self.brain_class
    }

    pub const fn homeostasis(&self) -> &HomeostaticSnapshot {
        &self.homeostasis
    }

    pub fn homeostasis_mut(&mut self) -> &mut HomeostaticSnapshot {
        &mut self.homeostasis
    }

    pub const fn memory_bank(&self) -> &MemoryBank {
        &self.memory_bank
    }

    pub const fn topological_map(&self) -> &TopologicalMap {
        self.topology_sidecar.map()
    }

    pub const fn development_state(&self) -> &DevelopmentState {
        &self.development_state
    }

    pub const fn sleep_state(&self) -> SleepState {
        self.sleep_controller.state()
    }

    pub fn pending_structural_edits(&self) -> &[StructuralEditBatch] {
        &self.pending_structural_edits
    }

    pub const fn diagnostics(&self) -> BrainTickDiagnostics {
        self.diagnostics
    }

    pub const fn neural_projection_schema(&self) -> &NeuralProjectionSchema {
        &self.neural_schema
    }

    pub fn initialize_neural_projection_schema(
        &mut self,
        schema: NeuralProjectionSchema,
    ) -> Result<(), ScaffoldContractError> {
        if self.next_sequence_id != 1 {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        schema.validate()?;
        if schema.brain_class_id != self.brain_class.id
            || schema.neuron_count != self.brain_class.neuron_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        self.neural_schema = schema;
        self.neural_state.projections = self.neural_schema.projections.clone();
        self.validate_ready(self.tick)
    }

    pub fn force_sleep(
        &mut self,
        tick: Tick,
        trigger: SleepTrigger,
    ) -> Result<SleepTransition, ScaffoldContractError> {
        self.sleep_controller.force_sleep(tick, trigger)
    }

    pub fn run_sleep_consolidation(
        &mut self,
        tick: Tick,
    ) -> Result<SleepConsolidationReport, ScaffoldContractError> {
        Tick::validate_monotonic(self.tick, tick)?;
        self.sleep_controller.validate_contract()?;
        self.neural_schema.validate()?;
        self.memory_bank
            .records_chronological()
            .iter()
            .try_for_each(|record| record.validate_contract())?;
        self.topology_sidecar.validate_contract()?;

        let consolidator = SleepConsolidator::new(self.sleep_controller.config())?;
        let neural = consolidator.consolidate_neural_schema(
            &mut self.neural_schema,
            &mut self.lifetime_traits,
            tick,
        )?;
        self.neural_state.projections = self.neural_schema.projections.clone();
        let memory = consolidator.compress_memory_bank(&mut self.memory_bank)?;
        let topology = consolidator.consolidate_topology_sidecar(&mut self.topology_sidecar, 1)?;
        let structural_edits = consolidator.generate_structural_edit_batch(
            &self.neural_schema,
            self.topology_sidecar.map(),
            tick,
        )?;
        self.pending_structural_edits.push(structural_edits.clone());
        self.development_state.sleep_cycle_count =
            self.development_state.sleep_cycle_count.saturating_add(1);
        self.development_state.consolidation_cycle_count = self
            .development_state
            .consolidation_cycle_count
            .saturating_add(1);
        self.development_state.last_sleep_tick = Some(tick);
        let report = SleepConsolidationReport {
            schema_version: crate::SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            tick,
            sleep_phase: self.sleep_controller.state().phase,
            traits: crate::TraitPromotionReport {
                promoted_count: neural.promoted_trait_count,
                ..crate::TraitPromotionReport::default()
            },
            neural,
            memory,
            topology,
            structural_edits,
        };
        report.validate_contract()?;
        Ok(report)
    }

    pub fn tick<S, A, O>(
        &mut self,
        input: BrainTickInput,
        sensory_adapter: &mut S,
        action_executor: &mut A,
        outcome_observer: &mut O,
    ) -> BrainTickOutput
    where
        S: ReferenceSensoryAdapter,
        A: ReferenceActionExecutor,
        O: ReferenceOutcomeObserver,
    {
        match self.try_tick(input, sensory_adapter, action_executor, outcome_observer) {
            Ok(output) => output,
            Err((error, diagnostics)) => BrainTickOutput::terminal(error, diagnostics),
        }
    }

    fn try_tick<S, A, O>(
        &mut self,
        input: BrainTickInput,
        sensory_adapter: &mut S,
        action_executor: &mut A,
        outcome_observer: &mut O,
    ) -> Result<BrainTickOutput, (ScaffoldContractError, BrainTickDiagnostics)>
    where
        S: ReferenceSensoryAdapter,
        A: ReferenceActionExecutor,
        O: ReferenceOutcomeObserver,
    {
        let mut diagnostics = BrainTickDiagnostics::default();
        fallible(&mut diagnostics, self.validate_ready(input.tick))?;
        fallible(&mut diagnostics, input.validate_contract(self.tick))?;
        if self.sleep_controller.state().phase != SleepPhase::Awake {
            return Ok(BrainTickOutput::sleep_idle(diagnostics));
        }

        let mut next_neural_state = self.neural_state.clone();
        let mut next_neural_schema = self.neural_schema.clone();
        let spmv_report = fallible(
            &mut diagnostics,
            cpu_spmv_projection(
                &next_neural_schema,
                &mut next_neural_state,
                NeuralDiagnostics::reference(),
            ),
        )?;
        diagnostics.observe_neural(spmv_report);
        let activation_report = fallible(
            &mut diagnostics,
            finalize_cpu_activations(&mut next_neural_state, NeuralActivationConfig::reference()),
        )?;
        diagnostics.observe_neural(activation_report);
        next_neural_state.update_metadata.tick = input.tick;

        let sensory = fallible(
            &mut diagnostics,
            sensory_adapter.gather_sensory(ReferenceSensoryRequest {
                organism_id: self.organism_id,
                tick: input.tick,
                body_pose: self.body.pose,
                body_velocity: self.body.velocity,
                homeostasis: self.homeostasis,
            }),
        )?;
        fallible(&mut diagnostics, sensory.validate_contract())?;

        let sequence_id = fallible(
            &mut diagnostics,
            ExperienceSequenceId::new(self.next_sequence_id)
                .ok_or(ScaffoldContractError::InvalidId),
        )?;
        let pre_action = fallible(
            &mut diagnostics,
            self.build_pre_action(
                sequence_id,
                input.tick,
                sensory,
                &input.proposals,
                input.action_duration,
                input.fallback_kind,
                MemoryExpectancySnapshot::neutral(),
            ),
        )?;

        let modulated_proposals = fallible(
            &mut diagnostics,
            self.modulate_baseline_proposals(&input.proposals, input.tick),
        )?;
        let trace_ref = fallible(
            &mut diagnostics,
            ActionArbitrationTraceRef::new(sequence_id.raw())
                .ok_or(ScaffoldContractError::InvalidActionDecision),
        )?;
        let fallback_confidence = fallible(&mut diagnostics, Confidence::new(0.25))?;
        let fallback_intensity = fallible(&mut diagnostics, Intensity::new(0.0))?;
        let decision = fallible(
            &mut diagnostics,
            cpu_reference_arbitrate(
                self.organism_id,
                &modulated_proposals,
                ActionArbitrationConfig {
                    min_score: input.min_action_score,
                    min_confidence: input.min_action_confidence,
                    default_duration_ticks: input.action_duration,
                    fallback_kind: input.fallback_kind,
                    fallback_confidence,
                    fallback_intensity,
                    trace_ref,
                    tie_breaker_seed: deterministic_tie_seed(
                        self.deterministic_seed,
                        input.tick,
                        sequence_id,
                    ),
                },
            ),
        )?;
        let status_after_arbitration = match decision.status {
            ActionDecisionStatus::Selected => BrainTickStatus::Normal,
            ActionDecisionStatus::FallbackSelected => BrainTickStatus::SafeIdle,
        };
        let decision_snapshot = fallible(
            &mut diagnostics,
            DecisionSnapshot::from_action_decision(
                sequence_id,
                input.tick,
                modulated_proposals,
                decision,
            ),
        )?;

        let execution = fallible(
            &mut diagnostics,
            action_executor.execute_action(&decision_snapshot.selected_action),
        )?;
        fallible(&mut diagnostics, execution.validate_contract())?;
        let observation = fallible(
            &mut diagnostics,
            outcome_observer.observe_outcome(ReferenceOutcomeRequest {
                command: &decision_snapshot.selected_action,
                execution: &execution,
                pre_action: &pre_action,
                decision: &decision_snapshot,
            }),
        )?;
        fallible(&mut diagnostics, observation.validate_contract())?;

        let outcome = fallible(
            &mut diagnostics,
            build_post_action(
                self.organism_id,
                sequence_id,
                next_tick(input.tick),
                execution,
                observation,
            ),
        )?;
        let patch = fallible(
            &mut diagnostics,
            ExperiencePatchBuilder::new(sequence_id)
                .record_pre_action(pre_action)
                .and_then(|builder| builder.record_decision(decision_snapshot))
                .and_then(|builder| builder.record_outcome(outcome))
                .and_then(ExperiencePatchBuilder::seal),
        )?;
        diagnostics.cpu_patch_allocations = diagnostics.cpu_patch_allocations.saturating_add(1);

        let mut staged_memory = self.memory_bank.clone();
        let memory_update = fallible(&mut diagnostics, staged_memory.insert_from_patch(&patch))?;
        diagnostics.memory_updates = diagnostics.memory_updates.saturating_add(1);
        let mut staged_topology = self.topology_sidecar.clone();
        let topology_receipt = staged_topology.observe_legacy_patch(&patch);
        let topology_update = topology_receipt.update;
        diagnostics.topology_updates = diagnostics
            .topology_updates
            .saturating_add(u32::from(topology_update.is_some()));
        let next_homeostasis = fallible(
            &mut diagnostics,
            self.homeostasis.advance(
                next_tick(input.tick),
                patch.outcome().homeostatic_delta,
                self.homeostatic_parameters,
            ),
        )?;

        if input.enable_learning_trace_update {
            let learning_rate_scale = fallible(
                &mut diagnostics,
                ChemistryModulation::learning_rate_scale(
                    &next_homeostasis,
                    self.homeostatic_parameters,
                ),
            )?;
            let oja_report = fallible(
                &mut diagnostics,
                update_oja_shadow_traces(
                    &mut next_neural_schema,
                    &next_neural_state,
                    OjaUpdateConfig {
                        learning_rate_scale,
                        ..OjaUpdateConfig::reference()
                    },
                ),
            )?;
            diagnostics.observe_neural(oja_report);
            diagnostics.learning_updates = diagnostics.learning_updates.saturating_add(1);
        }

        let packed_record = if input.pack_experience {
            let record = fallible(&mut diagnostics, ExperiencePacker::default().pack(&patch))?;
            diagnostics.packed_log_records = diagnostics.packed_log_records.saturating_add(1);
            Some(record)
        } else {
            None
        };

        let final_status = if execution.succeeded {
            status_after_arbitration
        } else {
            diagnostics.recoverable_action_failures =
                diagnostics.recoverable_action_failures.saturating_add(1);
            BrainTickStatus::RecoverableActionFailure
        };

        self.memory_bank = staged_memory;
        self.topology_sidecar = staged_topology;
        self.homeostasis = next_homeostasis;
        self.development_state.age_ticks = self.homeostasis.tick;
        self.neural_state = next_neural_state;
        self.neural_schema = next_neural_schema;
        self.action_state.record_execution(
            patch.decision().selected_action,
            input.tick,
            execution.succeeded,
        );
        self.tick = self.homeostasis.tick;
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
        self.diagnostics = diagnostics;

        Ok(BrainTickOutput {
            status: final_status,
            selected_action: Some(patch.decision().selected_action),
            experience_patch: Some(patch),
            packed_record,
            memory_update: Some(memory_update),
            topology_update,
            endocrine_update: Some(next_homeostasis),
            neural_report: NeuralUpdateReport {
                active_tiles: diagnostics.active_tiles,
                active_synapses: diagnostics.active_synapses,
                mask_skipped_tiles: diagnostics.supertiles_skipped,
                range_rejections: diagnostics.nan_or_range_rejections,
                ..NeuralUpdateReport::default()
            },
            diagnostics,
        })
    }

    fn validate_ready(&self, expected_tick: Tick) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.brain_class.validate()?;
        self.genome.validate_contract()?;
        self.development_state.validate_contract()?;
        self.body.validate_contract()?;
        self.homeostasis.validate_contract()?;
        self.neural_schema.validate()?;
        self.sleep_controller.validate_contract()?;
        self.lifetime_traits.validate_contract()?;
        for batch in &self.pending_structural_edits {
            batch.validate_contract()?;
        }
        self.topology_sidecar.validate_contract()?;
        validate_finite_slice(&self.neural_state.activations)?;
        validate_finite_slice(&self.neural_state.previous_activations)?;
        validate_finite_slice(&self.neural_state.accumulators)?;
        if self.tick != expected_tick || self.homeostasis.tick != expected_tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        if self.genome.brain_class_id != self.brain_class.id
            || self.neural_state.brain_class_id != self.brain_class.id
            || self.neural_schema.brain_class_id != self.brain_class.id
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn build_pre_action(
        &self,
        sequence_id: ExperienceSequenceId,
        tick: Tick,
        sensory: SensorySnapshot,
        proposals: &[ActionProposal],
        action_duration: DurationTicks,
        fallback_kind: ActionKind,
        memory_expectancy: MemoryExpectancySnapshot,
    ) -> Result<PreActionSnapshot, ScaffoldContractError> {
        let weight_split = WeightSplitContract::for_brain_class(
            self.brain_class.id,
            self.brain_class.max_active_synapses,
            self.brain_class.max_active_microtiles,
            self.genome.genetic_prior_seed,
        )?;
        let mut candidates = proposals
            .iter()
            .enumerate()
            .map(|(index, proposal)| {
                ActionCandidate::new(
                    u16::try_from(index)
                        .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                    proposal.action_id,
                    proposal.kind,
                    CandidateActionFamily::baseline_for_kind(proposal.kind),
                    CandidateObservationRef::None,
                    proposal.target,
                    CandidateFeatureVector::zero(),
                    proposal.confidence,
                    NormalizedScalar::new(0.0)?,
                    action_duration,
                    action_duration,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        if candidates.is_empty() {
            candidates.push(ActionCandidate::new(
                0,
                fallback_kind.canonical_id(),
                fallback_kind,
                CandidateActionFamily::baseline_for_kind(fallback_kind),
                CandidateObservationRef::None,
                crate::ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence::new(0.25)?,
                NormalizedScalar::new(0.0)?,
                action_duration,
                action_duration,
            )?);
        }
        let perception = PerceptionFrame::new(
            self.organism_id,
            tick,
            SensorProfile::PrivilegedAffordanceV1,
            sensory,
            BodySnapshot {
                pose: self.body.pose,
                velocity: self.body.velocity,
            },
            self.homeostasis,
            candidates,
            crate::SensorProfileProvenance::new(
                SensorProfile::PrivilegedAffordanceV1,
                crate::SensoryAbiVersion::CURRENT,
                tick,
            )?,
            Vec::new(),
        )?;
        PreActionSnapshot::from_heuristic_frame(
            sequence_id,
            perception,
            self.brain_class.clone(),
            self.genome.clone(),
            self.development_state.clone(),
            weight_split,
            memory_expectancy,
        )
    }

    fn modulate_baseline_proposals(
        &self,
        proposals: &[ActionProposal],
        tick: Tick,
    ) -> Result<Vec<ActionProposal>, ScaffoldContractError> {
        let salience_weight =
            ChemistryModulation::salience_weight(&self.homeostasis, self.homeostatic_parameters)?;
        let endocrine_delta =
            (self.homeostasis.hormones.dopamine - self.homeostasis.hormones.cortisol) * 0.05;

        proposals
            .iter()
            .copied()
            .map(|mut proposal| {
                let existing_bias = proposal.score_bias.map_or(0.0, |bias| bias.score_delta);
                let score_delta = existing_bias
                    + proposal.salience.raw() * salience_weight * 0.1
                    + endocrine_delta
                    + self.action_state.recent_penalty_for(&proposal, tick);
                crate::validate_finite(score_delta)?;
                let confidence = ChemistryModulation::motor_confidence(
                    proposal.confidence,
                    &self.homeostasis,
                    self.homeostatic_parameters,
                )?;
                proposal.confidence = confidence;
                proposal.score_bias = Some(ActionScoreBias {
                    source: ActionBiasSource::EndocrineDrive,
                    score_delta,
                });
                Ok(proposal)
            })
            .collect()
    }
}

fn build_post_action(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    outcome_tick: Tick,
    execution: ReferenceActionExecution,
    observation: ReferenceOutcomeObservation,
) -> Result<PostActionOutcome, ScaffoldContractError> {
    let mut outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        outcome_tick,
        observation.success && execution.succeeded,
        execution.physical,
        observation.homeostatic_delta,
        observation.reward_valence,
        observation.frustration_delta,
        observation.pain_delta,
        observation.energy_delta,
        observation.prediction_error,
    )?;
    outcome.contradiction_observed = observation.contradiction_observed || !execution.succeeded;
    outcome.validate_contract()?;
    Ok(outcome)
}

fn deterministic_tie_seed(seed: u64, tick: Tick, sequence_id: ExperienceSequenceId) -> u64 {
    seed ^ tick.raw().rotate_left(17) ^ sequence_id.raw().rotate_left(31)
}

fn next_tick(tick: Tick) -> Tick {
    Tick::new(tick.raw().saturating_add(1))
}

fn fallible<T>(
    diagnostics: &mut BrainTickDiagnostics,
    result: Result<T, ScaffoldContractError>,
) -> Result<T, (ScaffoldContractError, BrainTickDiagnostics)> {
    result.map_err(|error| {
        diagnostics.observe_error(&error);
        (error, *diagnostics)
    })
}

#[allow(dead_code)]
fn idle_physical_outcome() -> Result<PhysicalActionOutcome, ScaffoldContractError> {
    let outcome = PhysicalActionOutcome {
        contact: PhysicalContactKind::None,
        target_entity: None,
        displacement: Vec3f::ZERO,
        collision_normal: None,
        energy_cost: NormalizedScalar::new(0.0)?,
    };
    outcome.validate_contract()?;
    Ok(outcome)
}
