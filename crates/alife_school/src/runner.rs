//! v0 scaffold: simple headless curriculum runner.

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
}

impl HeadlessCurriculumRunner {
    pub const fn new(curriculum: Curriculum, actor: EmbodiedTeacherActor) -> Self {
        Self {
            curriculum,
            current_index: 0,
            completed_step_count: 0,
            planner: CurriculumTeacherPlanner::bounded_default(),
            actor,
        }
    }

    pub fn current_step(&self) -> Option<&CurriculumStep> {
        self.curriculum.steps.get(self.current_index)
    }

    pub fn completed_step_count(&self) -> usize {
        self.completed_step_count
    }

    pub fn dispatch_current(&self) -> Result<LessonDispatch, ScaffoldContractError> {
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
        Ok(LessonDispatch {
            lesson_id: step.lesson_id,
            perception_events,
        })
    }

    pub fn observe_verification(
        &mut self,
        verification: &LessonVerification,
    ) -> Result<bool, ScaffoldContractError> {
        let _ = self
            .current_step()
            .ok_or(ScaffoldContractError::InvalidId)?;
        if !verification.passed {
            return Ok(false);
        }
        self.completed_step_count = self.completed_step_count.saturating_add(1);
        self.current_index = (self.current_index + 1).min(self.curriculum.steps.len());
        Ok(true)
    }
}
