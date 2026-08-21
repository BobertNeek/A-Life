//! Engine-neutral orchestration seam for automatic GPU-authoritative sleep.

use alife_core::sleep::{SleepTransactionIdentity, SleepTransactionState, SleepWorkReceipt};
use alife_core::{
    ActionId, BodyEventDelta, ConsolidationDriverEvent, ConsolidationIntent, ConsolidationState,
    HomeostaticParameters, HomeostaticSnapshot, OrganismId, ScaffoldContractError,
    SleepConsolidationConfig, SleepController, SleepPhase, SleepState, SleepTransition,
    SleepTrigger, Tick, Validate,
};
use alife_world::{OrganismSleepInput, WorldOrganismRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleepWorkDue(u8);

impl SleepWorkDue {
    pub const REPLAY: Self = Self(1 << 0);
    pub const FAST_TO_LIFETIME: Self = Self(1 << 1);
    pub const PREDICTOR: Self = Self(1 << 2);
    pub const CONCEPT_GAP: Self = Self(1 << 3);
    pub const STRUCTURAL_GROWTH_PRUNING: Self = Self(1 << 4);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    const fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleepSubsystemCadence {
    pub replay_ticks: u64,
    pub fast_to_lifetime_ticks: u64,
    pub predictor_ticks: u64,
    pub concept_gap_ticks: u64,
    pub structural_growth_pruning_ticks: u64,
}

impl SleepSubsystemCadence {
    pub const fn reference() -> Self {
        Self {
            replay_ticks: 1,
            fast_to_lifetime_ticks: 2,
            predictor_ticks: 2,
            concept_gap_ticks: 4,
            structural_growth_pruning_ticks: 4,
        }
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.replay_ticks == 0
            || self.fast_to_lifetime_ticks == 0
            || self.predictor_ticks == 0
            || self.concept_gap_ticks == 0
            || self.structural_growth_pruning_ticks == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleepPhaseReceipt {
    pub phase: SleepPhase,
    pub cycle_id: u64,
    pub tick: Tick,
    pub due_work: SleepWorkDue,
    pub work_units: u64,
    pub cumulative_work_units: u64,
    pub sealed: bool,
}

pub trait GpuSleepConsolidationDriver {
    fn progress(
        &mut self,
        organism_id: OrganismId,
        state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, ScaffoldContractError>;

    /// Runs the due bounded core sleep transaction.
    ///
    /// Implementations should call `SleepConsolidator::run_bounded_transaction`
    /// and return its validated receipt. The default keeps the legacy GPU-only
    /// caller source-compatible; the organism-aware path requires a receipt.
    fn run_bounded_sleep_transaction(
        &mut self,
        _organism_id: OrganismId,
        _state: SleepState,
        _homeostasis: &HomeostaticSnapshot,
        _tick: Tick,
        _due_work: SleepWorkDue,
    ) -> Result<Option<SleepWorkReceipt>, ScaffoldContractError> {
        Ok(None)
    }

    /// Optional durable transaction state supplied by a host that stages the
    /// full CPU/GPU sleep result.  Older drivers remain source-compatible and
    /// report no state until they adopt the shared transaction boundary.
    fn sleep_transaction_state(&self) -> Option<SleepTransactionState> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSleepScheduleEvent {
    pub tick: Tick,
    pub phase: SleepPhase,
    pub cycle_id: u64,
    pub transition: Option<SleepTransition>,
    pub consolidation_kind_raw: u16,
    pub selected_action: Option<ActionId>,
    pub motor_eligible: bool,
    pub sleep_work_units: u64,
    pub phase_receipt: SleepPhaseReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepScheduler {
    controller: SleepController,
    cadence: SleepSubsystemCadence,
    last_emitted_intent_cycle: Option<u64>,
    last_sleep_work_cycle: Option<u64>,
    last_sleep_work_ticks: [Option<Tick>; 5],
    transaction: SleepTransactionState,
}

impl GpuSleepScheduler {
    pub fn new(config: SleepConsolidationConfig) -> Result<Self, ScaffoldContractError> {
        Self::with_cadence(config, SleepSubsystemCadence::reference())
    }

    pub fn with_cadence(
        config: SleepConsolidationConfig,
        cadence: SleepSubsystemCadence,
    ) -> Result<Self, ScaffoldContractError> {
        cadence.validate()?;
        Ok(Self {
            controller: SleepController::new(config)?,
            cadence,
            last_emitted_intent_cycle: None,
            last_sleep_work_cycle: None,
            last_sleep_work_ticks: [None; 5],
            transaction: SleepTransactionState::Idle,
        })
    }

    pub fn restore(
        config: SleepConsolidationConfig,
        state: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        Self::restore_with_cadence(config, state, SleepSubsystemCadence::reference())
    }

    pub fn restore_with_cadence(
        config: SleepConsolidationConfig,
        state: SleepState,
        cadence: SleepSubsystemCadence,
    ) -> Result<Self, ScaffoldContractError> {
        Self::restore_with_cadence_and_transaction(
            config,
            state,
            cadence,
            SleepTransactionState::Idle,
        )
    }

    pub fn restore_with_cadence_and_transaction(
        config: SleepConsolidationConfig,
        state: SleepState,
        cadence: SleepSubsystemCadence,
        transaction: SleepTransactionState,
    ) -> Result<Self, ScaffoldContractError> {
        cadence.validate()?;
        let controller = SleepController::restore(config, state)?;
        transaction.validate_contract()?;
        let last_emitted_intent_cycle = if state.phase == SleepPhase::Consolidating
            && state.consolidation != ConsolidationState::None
        {
            Some(state.active_cycle_id)
        } else {
            None
        };
        Ok(Self {
            controller,
            cadence,
            last_emitted_intent_cycle,
            last_sleep_work_cycle: None,
            last_sleep_work_ticks: [None; 5],
            transaction,
        })
    }

    pub const fn state(&self) -> SleepState {
        self.controller.state()
    }

    pub const fn cadence(&self) -> SleepSubsystemCadence {
        self.cadence
    }

    pub const fn sleep_transaction_state(&self) -> SleepTransactionState {
        self.transaction
    }

    pub fn begin_sleep_transaction(
        &mut self,
        identity: SleepTransactionIdentity,
        snapshot_digest: [u64; 4],
        staged_digest: [u64; 4],
    ) -> Result<(), ScaffoldContractError> {
        identity.validate_contract()?;
        let next = SleepTransactionState::Pending {
            identity,
            snapshot_digest,
            staged_digest,
        };
        next.validate_contract()?;
        self.transaction = next;
        Ok(())
    }

    pub fn commit_sleep_transaction(
        &mut self,
        identity: SleepTransactionIdentity,
        commit_digest: [u64; 4],
    ) -> Result<(), ScaffoldContractError> {
        if !matches!(
            self.transaction,
            SleepTransactionState::Pending { identity: current, .. } if current == identity
        ) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let next = SleepTransactionState::Committed {
            identity,
            commit_digest,
        };
        next.validate_contract()?;
        self.transaction = next;
        Ok(())
    }

    pub fn interrupt_sleep_transaction(
        &mut self,
        identity: SleepTransactionIdentity,
        snapshot_digest: [u64; 4],
        reason_code: u16,
    ) -> Result<(), ScaffoldContractError> {
        identity.validate_contract()?;
        let next = SleepTransactionState::Interrupted {
            identity,
            snapshot_digest,
            reason_code,
        };
        next.validate_contract()?;
        self.transaction = next;
        Ok(())
    }

    pub fn force_recovery_sleep(
        &mut self,
        tick: Tick,
    ) -> Result<SleepTransition, ScaffoldContractError> {
        self.controller
            .force_sleep(tick, SleepTrigger::RecoveryProtocol)
    }

    pub fn scheduled_tick<D: GpuSleepConsolidationDriver>(
        &mut self,
        organism_id: OrganismId,
        homeostasis: &HomeostaticSnapshot,
        parameters: HomeostaticParameters,
        tick: Tick,
        driver: &mut D,
    ) -> Result<GpuSleepScheduleEvent, ScaffoldContractError> {
        let phase_before = self.controller.state().phase;
        let transition = if phase_before == SleepPhase::Awake {
            self.controller
                .evaluate_homeostasis(homeostasis, parameters, tick)?
        } else {
            self.controller.advance(tick)?
        };

        self.progress_driver(organism_id, driver)?;

        let state = self.controller.state();
        if state.phase == SleepPhase::Awake {
            self.last_emitted_intent_cycle = None;
            self.reset_sleep_work_schedule();
        }
        Ok(self.event(
            tick,
            transition,
            state,
            state.phase == SleepPhase::Awake,
            SleepWorkDue::empty(),
            0,
            0,
            false,
        ))
    }

    /// Schedules sleep from the world-owned biological record.
    ///
    /// The world record advances biology once for the requested tick. The
    /// existing GPU progress path still runs first, then the due bounded core
    /// transaction is consumed as one work receipt and sealed back into the
    /// organism for persistence and presentation.
    pub fn scheduled_tick_with_organism<D: GpuSleepConsolidationDriver>(
        &mut self,
        organism: &mut WorldOrganismRecord,
        parameters: HomeostaticParameters,
        tick: Tick,
        driver: &mut D,
        explicit_sleep_behavior: bool,
    ) -> Result<GpuSleepScheduleEvent, ScaffoldContractError> {
        let before = organism.authoritative_sleep_input()?;
        if !before.lifecycle.is_alive() {
            return Err(ScaffoldContractError::InvalidId);
        }

        let phase_before = self.controller.state().phase;
        organism.advance_biology_once(
            tick,
            BodyEventDelta {
                sleep_recovery: if phase_before != SleepPhase::Awake || before.body_sleeping {
                    1.0
                } else {
                    0.0
                },
                ..BodyEventDelta::zero()
            },
        )?;
        let input = organism.authoritative_sleep_input()?;
        if input.biological_tick != tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }

        let transition = self.advance_authoritative(&input, parameters, tick)?;
        self.progress_driver(input.organism_id, driver)?;

        let state = self.controller.state();
        let due_work = self.sleep_work_due(state, tick);
        let work_units = if due_work.is_empty() {
            0
        } else {
            let work_homeostasis =
                Self::homeostasis_for_due_work(&input.homeostasis, self.controller.config());
            let receipt = driver
                .run_bounded_sleep_transaction(
                    input.organism_id,
                    state,
                    &work_homeostasis,
                    tick,
                    due_work,
                )?
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            receipt.validate_contract()?;
            if let Some(transaction) = driver.sleep_transaction_state() {
                transaction.validate_contract()?;
                self.transaction = transaction;
            }
            self.commit_sleep_work(state.active_cycle_id, tick, due_work);
            receipt.work_units
        };

        if state.phase == SleepPhase::Awake {
            self.last_emitted_intent_cycle = None;
            self.reset_sleep_work_schedule();
        }

        let cycle_id = Self::cycle_id(state);
        organism.seal_sleep_phase(state.phase, cycle_id, tick, work_units)?;
        let seal = organism.sleep_seal();
        Ok(self.event(
            tick,
            transition,
            state,
            state.phase == SleepPhase::Awake || explicit_sleep_behavior,
            due_work,
            work_units,
            seal.work_units,
            true,
        ))
    }

    fn homeostasis_for_due_work(
        homeostasis: &HomeostaticSnapshot,
        config: SleepConsolidationConfig,
    ) -> HomeostaticSnapshot {
        let mut effective = *homeostasis;
        effective.drives.fatigue = effective.drives.fatigue.max(config.fatigue_threshold.raw());
        effective.hormones.sleep_pressure = effective
            .hormones
            .sleep_pressure
            .max(config.sleep_pressure_threshold.raw());
        effective
    }

    fn advance_authoritative(
        &mut self,
        input: &OrganismSleepInput,
        parameters: HomeostaticParameters,
        tick: Tick,
    ) -> Result<Option<SleepTransition>, ScaffoldContractError> {
        let phase = self.controller.state().phase;
        if phase == SleepPhase::Awake {
            if input.energy <= 0.20 {
                self.controller
                    .force_sleep(tick, SleepTrigger::RecoveryProtocol)
                    .map(Some)
            } else {
                self.controller
                    .evaluate_homeostasis(&input.homeostasis, parameters, tick)
            }
        } else if phase == SleepPhase::Waking {
            let config = self.controller.config();
            let wake_ready = input.energy >= 0.35
                && input.homeostasis.drives.fatigue < config.fatigue_threshold.raw()
                && input.homeostasis.hormones.sleep_pressure
                    < config.sleep_pressure_threshold.raw();
            if wake_ready {
                self.controller.advance(tick)
            } else {
                Ok(None)
            }
        } else {
            self.controller.advance(tick)
        }
    }

    fn progress_driver<D: GpuSleepConsolidationDriver>(
        &mut self,
        organism_id: OrganismId,
        driver: &mut D,
    ) -> Result<(), ScaffoldContractError> {
        let state_before_driver = self.controller.state();
        let intent = if state_before_driver.phase == SleepPhase::Consolidating
            && state_before_driver.consolidation == ConsolidationState::None
            && self.last_emitted_intent_cycle != Some(state_before_driver.active_cycle_id)
        {
            Some(ConsolidationIntent {
                cycle_id: state_before_driver.active_cycle_id,
            })
        } else {
            None
        };

        if state_before_driver.phase == SleepPhase::Consolidating {
            let progress = driver.progress(organism_id, state_before_driver, intent)?;
            if intent.is_some() && progress.is_none() {
                return Err(ScaffoldContractError::MissingPhaseData);
            }
            if let Some(progress) = progress {
                self.controller.apply_consolidation_driver_event(progress)?;
                if let Some(intent) = intent {
                    self.last_emitted_intent_cycle = Some(intent.cycle_id);
                }
            }
        }
        Ok(())
    }

    fn sleep_work_due(&self, state: SleepState, tick: Tick) -> SleepWorkDue {
        if state.phase != SleepPhase::Consolidating
            || state.consolidation == ConsolidationState::None
        {
            return SleepWorkDue::empty();
        }

        let previous = if self.last_sleep_work_cycle == Some(state.active_cycle_id) {
            self.last_sleep_work_ticks
        } else {
            [None; 5]
        };
        let periods = [
            self.cadence.replay_ticks,
            self.cadence.fast_to_lifetime_ticks,
            self.cadence.predictor_ticks,
            self.cadence.concept_gap_ticks,
            self.cadence.structural_growth_pruning_ticks,
        ];
        let flags = [
            SleepWorkDue::REPLAY,
            SleepWorkDue::FAST_TO_LIFETIME,
            SleepWorkDue::PREDICTOR,
            SleepWorkDue::CONCEPT_GAP,
            SleepWorkDue::STRUCTURAL_GROWTH_PRUNING,
        ];
        let mut due = SleepWorkDue::empty();
        for index in 0..periods.len() {
            if previous[index].map_or(true, |last| {
                tick.raw().saturating_sub(last.raw()) >= periods[index]
            }) {
                due.insert(flags[index]);
            }
        }
        due
    }

    fn commit_sleep_work(&mut self, cycle_id: u64, tick: Tick, due: SleepWorkDue) {
        if self.last_sleep_work_cycle != Some(cycle_id) {
            self.last_sleep_work_cycle = Some(cycle_id);
            self.last_sleep_work_ticks = [None; 5];
        }
        let flags = [
            SleepWorkDue::REPLAY,
            SleepWorkDue::FAST_TO_LIFETIME,
            SleepWorkDue::PREDICTOR,
            SleepWorkDue::CONCEPT_GAP,
            SleepWorkDue::STRUCTURAL_GROWTH_PRUNING,
        ];
        for index in 0..flags.len() {
            if due.contains(flags[index]) {
                self.last_sleep_work_ticks[index] = Some(tick);
            }
        }
    }

    fn reset_sleep_work_schedule(&mut self) {
        self.last_sleep_work_cycle = None;
        self.last_sleep_work_ticks = [None; 5];
    }

    fn event(
        &self,
        tick: Tick,
        transition: Option<SleepTransition>,
        state: SleepState,
        motor_eligible: bool,
        due_work: SleepWorkDue,
        work_units: u64,
        cumulative_work_units: u64,
        sealed: bool,
    ) -> GpuSleepScheduleEvent {
        let cycle_id = Self::cycle_id(state);
        GpuSleepScheduleEvent {
            tick,
            phase: state.phase,
            cycle_id,
            transition,
            consolidation_kind_raw: state.consolidation.kind_raw(),
            selected_action: None,
            motor_eligible,
            sleep_work_units: work_units,
            phase_receipt: SleepPhaseReceipt {
                phase: state.phase,
                cycle_id,
                tick,
                due_work,
                work_units,
                cumulative_work_units,
                sealed,
            },
        }
    }

    const fn cycle_id(state: SleepState) -> u64 {
        if state.active_cycle_id == 0 {
            state.last_consolidated_cycle_id
        } else {
            state.active_cycle_id
        }
    }
}
