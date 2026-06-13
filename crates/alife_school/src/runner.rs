//! v0 scaffold: simple headless curriculum runner.

use alife_core::ScaffoldContractError;

use crate::{Curriculum, CurriculumStep, LessonId, LessonVerification, TeacherPerceptualEvent};

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
}

impl HeadlessCurriculumRunner {
    pub const fn new(curriculum: Curriculum) -> Self {
        Self {
            curriculum,
            current_index: 0,
            completed_step_count: 0,
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
        Ok(LessonDispatch {
            lesson_id: step.lesson_id,
            perception_events: step.prompt_cues.clone(),
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
