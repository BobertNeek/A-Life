//! Reporting only: a bounded waking trace cannot establish what happens after sleep.
use alife_core::SleepPhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationEvaluationStopReason {
    TickBudgetExhausted,
    LessonBoundary,
    NeuralSleep,
    TerminalDeath,
    MissingWakingCapture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationEvaluationOutcome {
    Completed,
    /// The measured criterion was false within this observation window only.
    NotCompleted,
    /// No post-sleep waking decision was observed; inability to wake is unproven.
    SleepCensored,
    TerminalDeath,
    MissingWakingCapture,
}

/// Additional evidence beside the unchanged `lesson_completed` acceptance flag.
/// Historical receipts have no observation and must remain unknown.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoundationEvaluationObservation {
    pub organism_id: alife_core::OrganismId,
    pub stop_reason: FoundationEvaluationStopReason,
    pub lesson_outcome: Option<FoundationEvaluationOutcome>,
    pub requested_ticks: usize,
    pub tick_calls: usize,
    pub captured_waking_steps: usize,
    pub last_tick_world_before: u64,
    pub final_world_tick: u64,
    pub neural_phase_before_last_tick: Option<SleepPhase>,
    pub final_neural_phase: Option<SleepPhase>,
    /// Ordinary Rest may set this while the neural scheduler is still Awake.
    pub final_body_sleeping: Option<bool>,
    pub terminal_death_tick: Option<u64>,
}

impl FoundationEvaluationObservation {
    pub(super) fn new(requested_ticks: usize, organism_id: alife_core::OrganismId) -> Self {
        Self {
            organism_id,
            stop_reason: FoundationEvaluationStopReason::TickBudgetExhausted,
            lesson_outcome: None,
            requested_ticks,
            tick_calls: 0,
            captured_waking_steps: 0,
            last_tick_world_before: 0,
            final_world_tick: 0,
            neural_phase_before_last_tick: None,
            final_neural_phase: None,
            final_body_sleeping: None,
            terminal_death_tick: None,
        }
    }

    pub(super) fn before_tick(&mut self, world: &alife_world::HeadlessWorld) {
        self.tick_calls += 1;
        self.last_tick_world_before = world.tick().raw();
        self.neural_phase_before_last_tick = world
            .organism_registry()
            .get(self.organism_id)
            .map(|record| record.sleep_phase());
    }

    pub(super) fn finish(
        &mut self,
        world: &alife_world::HeadlessWorld,
        retirement_death_tick: Option<u64>,
        captured_waking_steps: usize,
        lesson_completed: Option<bool>,
    ) {
        let record = world.organism_registry().get(self.organism_id);
        self.captured_waking_steps = captured_waking_steps;
        self.final_world_tick = world.tick().raw();
        self.final_neural_phase = record.map(|record| record.sleep_phase());
        self.final_body_sleeping = record.map(|record| record.biochemistry().body.sleeping);
        self.terminal_death_tick = record
            .and_then(|record| record.lifecycle().death_tick())
            .map(alife_core::Tick::raw)
            .or(retirement_death_tick);
        self.classify(lesson_completed);
    }

    pub(super) fn classify(&mut self, lesson_completed: Option<bool>) {
        use FoundationEvaluationOutcome as Outcome;
        use FoundationEvaluationStopReason as Stop;
        if self.terminal_death_tick.is_some() {
            self.stop_reason = Stop::TerminalDeath;
        } else if self.stop_reason == Stop::MissingWakingCapture
            && self.final_world_tick > self.last_tick_world_before
            && [self.neural_phase_before_last_tick, self.final_neural_phase]
                .into_iter()
                .flatten()
                .any(|phase| phase != SleepPhase::Awake)
        {
            // Also covers a Waking -> Awake tick, which is not dispatchable.
            // Body sleep alone, absent state, or no world progress is insufficient.
            self.stop_reason = Stop::NeuralSleep;
        }
        self.lesson_outcome = lesson_completed.map(|completed| match self.stop_reason {
            Stop::TerminalDeath => Outcome::TerminalDeath,
            Stop::MissingWakingCapture => Outcome::MissingWakingCapture,
            _ if completed => Outcome::Completed,
            Stop::NeuralSleep => Outcome::SleepCensored,
            Stop::TickBudgetExhausted | Stop::LessonBoundary => Outcome::NotCompleted,
        });
    }
}

pub(super) fn write_lesson_receipt(
    output: &std::path::Path,
    observation: &FoundationEvaluationObservation,
    mut document: serde_json::Value,
) -> super::Result<()> {
    let filename =
        if document["lesson_completed"] == false || observation.captured_waking_steps == 0 {
            "lesson-diagnostic.json"
        } else {
            "lesson-trace.json"
        };
    document["evaluation_observation"] = serde_json::to_value(observation)?;
    alife_storage::write(output.join(filename), serde_json::to_vec_pretty(&document)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_capture() -> FoundationEvaluationObservation {
        FoundationEvaluationObservation {
            stop_reason: FoundationEvaluationStopReason::MissingWakingCapture,
            tick_calls: 4,
            captured_waking_steps: 3,
            last_tick_world_before: 103,
            final_world_tick: 104,
            neural_phase_before_last_tick: Some(SleepPhase::Awake),
            final_neural_phase: Some(SleepPhase::Awake),
            final_body_sleeping: Some(true),
            ..FoundationEvaluationObservation::new(16, alife_core::OrganismId(1))
        }
    }

    #[test]
    fn neural_sleep_requires_scheduler_evidence_and_world_progress() {
        use FoundationEvaluationOutcome as Outcome;
        for phase in [
            SleepPhase::EnteringSleep,
            SleepPhase::Consolidating,
            SleepPhase::ForcedRecoverySleep,
            SleepPhase::Waking,
        ] {
            for waking in [false, true] {
                let mut observed = missing_capture();
                observed.neural_phase_before_last_tick = Some(phase);
                observed.final_neural_phase = Some(if waking { SleepPhase::Awake } else { phase });
                observed.classify(Some(false));
                assert_eq!(observed.lesson_outcome, Some(Outcome::SleepCensored));
                let encoded = serde_json::to_value(&observed).unwrap();
                assert_eq!(encoded["stop_reason"], "neural_sleep");
                assert_eq!(encoded["lesson_outcome"], "sleep_censored");
                assert_eq!(
                    serde_json::from_value::<FoundationEvaluationObservation>(encoded).unwrap(),
                    observed
                );
            }
        }
        for phase in [
            None,
            Some(SleepPhase::Awake),
            Some(SleepPhase::Consolidating),
        ] {
            let mut observed = missing_capture();
            observed.neural_phase_before_last_tick = phase;
            observed.final_neural_phase = phase;
            if phase == Some(SleepPhase::Consolidating) {
                observed.final_world_tick = observed.last_tick_world_before;
            }
            observed.classify(Some(false));
            assert_eq!(observed.lesson_outcome, Some(Outcome::MissingWakingCapture));
        }
    }

    #[test]
    fn terminal_death_takes_precedence_over_sleep_and_a_completed_criterion() {
        for completed in [false, true] {
            let mut observed = missing_capture();
            observed.final_neural_phase = Some(SleepPhase::ForcedRecoverySleep);
            observed.terminal_death_tick = Some(104);
            observed.classify(Some(completed));
            assert_eq!(
                observed.stop_reason,
                FoundationEvaluationStopReason::TerminalDeath
            );
            assert_eq!(
                observed.lesson_outcome,
                Some(FoundationEvaluationOutcome::TerminalDeath)
            );
        }
    }

    #[test]
    fn measured_noncompletion_and_success_remain_distinct_from_censoring() {
        for stop in [
            FoundationEvaluationStopReason::TickBudgetExhausted,
            FoundationEvaluationStopReason::LessonBoundary,
        ] {
            let mut observed = missing_capture();
            observed.stop_reason = stop;
            for (completed, outcome) in [
                (Some(false), Some(FoundationEvaluationOutcome::NotCompleted)),
                (Some(true), Some(FoundationEvaluationOutcome::Completed)),
                (None, None),
            ] {
                observed.classify(completed);
                assert_eq!(observed.stop_reason, stop);
                assert_eq!(observed.lesson_outcome, outcome);
            }
        }
    }
}
