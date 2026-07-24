//! Engine-neutral orchestration seam for automatic GPU-authoritative sleep.

use alife_core::{
    ActionId, ConsolidationDriverEvent, ConsolidationIntent, ConsolidationState,
    HomeostaticParameters, HomeostaticSnapshot, OrganismId, ScaffoldContractError,
    SleepConsolidationConfig, SleepController, SleepPhase, SleepState, SleepTransition, Tick,
};

pub trait GpuSleepConsolidationDriver {
    fn progress(
        &mut self,
        organism_id: OrganismId,
        state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSleepScheduleEvent {
    pub tick: Tick,
    pub phase: SleepPhase,
    pub cycle_id: u64,
    pub transition: Option<SleepTransition>,
    pub consolidation_kind_raw: u16,
    pub selected_action: Option<ActionId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepScheduler {
    controller: SleepController,
    last_emitted_intent_cycle: Option<u64>,
}

impl GpuSleepScheduler {
    pub fn new(config: SleepConsolidationConfig) -> Result<Self, ScaffoldContractError> {
        Ok(Self {
            controller: SleepController::new(config)?,
            last_emitted_intent_cycle: None,
        })
    }

    pub fn restore(
        config: SleepConsolidationConfig,
        state: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        let controller = SleepController::restore(config, state)?;
        let last_emitted_intent_cycle = if state.phase == SleepPhase::Consolidating
            && state.consolidation != ConsolidationState::None
        {
            Some(state.active_cycle_id)
        } else {
            None
        };
        Ok(Self {
            controller,
            last_emitted_intent_cycle,
        })
    }

    pub const fn state(&self) -> SleepState {
        self.controller.state()
    }

    pub fn force_recovery_sleep(
        &mut self,
        tick: Tick,
    ) -> Result<SleepTransition, ScaffoldContractError> {
        self.controller
            .force_sleep(tick, alife_core::SleepTrigger::RecoveryProtocol)
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

        let state = self.controller.state();
        if state.phase == SleepPhase::Awake {
            self.last_emitted_intent_cycle = None;
        }
        Ok(GpuSleepScheduleEvent {
            tick,
            phase: state.phase,
            cycle_id: if state.active_cycle_id == 0 {
                state.last_consolidated_cycle_id
            } else {
                state.active_cycle_id
            },
            transition,
            consolidation_kind_raw: state.consolidation.kind_raw(),
            selected_action: None,
        })
    }
}
