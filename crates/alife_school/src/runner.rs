//! v0 scaffold: simple headless curriculum runner.

use std::cell::Cell;

use alife_core::ScaffoldContractError;

use crate::{
    Curriculum, CurriculumStep, CurriculumTeacherPlanner, EmbodiedTeacherActor, LessonId,
    LessonVerification, PlannerVisibleState, TeacherPerceptualEvent, TeacherPlanner,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LessonDispatch {
    pub lesson_id: LessonId,
    pub perception_events: Vec<TeacherPerceptualEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessCurriculumRunner {
    curriculum: Curriculum,
    current_index: usize,
    completed_step_count: usize,
    planner: CurriculumTeacherPlanner,
    actor: EmbodiedTeacherActor,
    dispatched_lesson: Cell<Option<LessonId>>,
}

impl HeadlessCurriculumRunner {
    pub const fn new(curriculum: Curriculum, actor: EmbodiedTeacherActor) -> Self {
        Self {
            curriculum,
            current_index: 0,
            completed_step_count: 0,
            planner: CurriculumTeacherPlanner::bounded_default(),
            actor,
            dispatched_lesson: Cell::new(None),
        }
    }

    pub fn current_step(&self) -> Option<&CurriculumStep> {
        self.curriculum.steps.get(self.current_index)
    }

    pub fn completed_step_count(&self) -> usize {
        self.completed_step_count
    }

    pub fn dispatch_current(&self) -> Result<LessonDispatch, ScaffoldContractError> {
        if self.curriculum.schema_version != crate::TEACHER_SCHOOL_SCHEMA_VERSION {
            return Err(ScaffoldContractError::IncompatibleAbi {
                kind: alife_core::SchemaKind::TeacherSchool,
                expected: crate::TEACHER_SCHOOL_SCHEMA_VERSION,
                actual: self.curriculum.schema_version,
            });
        }
        if !self.curriculum.lesson_ids_are_unique() {
            return Err(ScaffoldContractError::InvalidId);
        }
        let step = self
            .current_step()
            .ok_or(ScaffoldContractError::InvalidId)?;
        let plan = self.planner.plan(
            step,
            PlannerVisibleState {
                developmental_stage_raw: 0,
                observable_success: false,
                coarse_homeostatic_stress: alife_core::NormalizedScalar::new(0.0)?,
                uncertainty: alife_core::NormalizedScalar::new(0.5)?,
            },
        )?;
        let perception_events = self.actor.enact_plan(&plan)?;
        self.dispatched_lesson.set(Some(step.lesson_id));
        Ok(LessonDispatch {
            lesson_id: step.lesson_id,
            perception_events,
        })
    }

    pub fn observe_verification(
        &mut self,
        verification: &LessonVerification,
    ) -> Result<bool, ScaffoldContractError> {
        let step = self
            .current_step()
            .ok_or(ScaffoldContractError::InvalidId)?;
        if self.dispatched_lesson.get() != Some(step.lesson_id)
            || !verification.passed
            || !verification.failed_checks.is_empty()
            || step
                .verifier_checks
                .iter()
                .any(|check| !verification.observed_checks.contains(check))
        {
            return Ok(false);
        }
        self.completed_step_count = self.completed_step_count.saturating_add(1);
        self.current_index = (self.current_index + 1).min(self.curriculum.steps.len());
        self.dispatched_lesson.set(None);
        Ok(true)
    }
}
