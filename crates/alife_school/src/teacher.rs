//! Read-only lesson planning and grounded embodied teaching contracts.

use alife_core::{
    Confidence, NormalizedScalar, ScaffoldContractError, SchemaVersions, TeacherPerceptionChannel,
    WorldEntityId,
};

use crate::{CurriculumStep, LessonId};

pub const TEACHER_SCHOOL_SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.teacher_school.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherRole {
    Tutor,
    Examiner,
    Critic,
    CurriculumPlanner,
    Verifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherInputKind {
    SpokenToken,
    Gesture,
    ObjectHighlight,
    SocialFeedback,
    SocialApproval,
    SocialDisapproval,
}

impl TeacherInputKind {
    pub const PERCEPTION_ONLY: [Self; 6] = [
        Self::SpokenToken,
        Self::Gesture,
        Self::ObjectHighlight,
        Self::SocialFeedback,
        Self::SocialApproval,
        Self::SocialDisapproval,
    ];

    pub const fn is_perceptual(&self) -> bool {
        matches!(
            self,
            Self::SpokenToken
                | Self::Gesture
                | Self::ObjectHighlight
                | Self::SocialFeedback
                | Self::SocialApproval
                | Self::SocialDisapproval
        )
    }

    pub const fn channel(self) -> TeacherPerceptionChannel {
        match self {
            Self::SpokenToken => TeacherPerceptionChannel::Hearing,
            Self::Gesture => TeacherPerceptionChannel::Gesture,
            Self::ObjectHighlight => TeacherPerceptionChannel::Object,
            Self::SocialFeedback | Self::SocialApproval | Self::SocialDisapproval => {
                TeacherPerceptionChannel::Vision
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackPolarity {
    Praise,
    Correction,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeacherPerceptualEvent {
    pub schema_version: u16,
    pub lesson_id: LessonId,
    pub input_kind: TeacherInputKind,
    pub channel: TeacherPerceptionChannel,
    pub token_id: Option<u32>,
    pub gesture_id: Option<u32>,
    pub object_entity: Option<WorldEntityId>,
    pub feedback: Option<FeedbackPolarity>,
    pub salience: NormalizedScalar,
    pub confidence: Confidence,
    pub teacher_entity: WorldEntityId,
    actor_seal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeacherAct {
    pub lesson_id: LessonId,
    pub input_kind: TeacherInputKind,
    pub token_id: Option<u32>,
    pub gesture_id: Option<u32>,
    pub object_entity: Option<WorldEntityId>,
    pub feedback: Option<FeedbackPolarity>,
    pub salience: NormalizedScalar,
    pub confidence: Confidence,
}

impl TeacherAct {
    pub fn spoken_token(lesson_id: LessonId, token_id: u32) -> Self {
        Self::new(lesson_id, TeacherInputKind::SpokenToken)
            .with_token_id(token_id)
            .with_confidence(Confidence(0.9))
    }

    pub fn gesture(lesson_id: LessonId, gesture_id: u32) -> Self {
        Self::new(lesson_id, TeacherInputKind::Gesture)
            .with_gesture_id(gesture_id)
            .with_confidence(Confidence(0.85))
    }

    pub fn object_highlight(
        lesson_id: LessonId,
        object_entity: WorldEntityId,
        salience: NormalizedScalar,
    ) -> Self {
        Self::new(lesson_id, TeacherInputKind::ObjectHighlight)
            .with_object_entity(object_entity)
            .with_salience(salience)
    }

    pub fn social_feedback(
        lesson_id: LessonId,
        feedback: FeedbackPolarity,
        confidence: Confidence,
    ) -> Self {
        Self::new(lesson_id, TeacherInputKind::SocialFeedback)
            .with_feedback(feedback)
            .with_confidence(confidence)
    }

    pub fn social_approval(lesson_id: LessonId, salience: NormalizedScalar) -> Self {
        Self::new(lesson_id, TeacherInputKind::SocialApproval)
            .with_feedback(FeedbackPolarity::Praise)
            .with_salience(salience)
    }

    pub fn social_disapproval(lesson_id: LessonId, salience: NormalizedScalar) -> Self {
        Self::new(lesson_id, TeacherInputKind::SocialDisapproval)
            .with_feedback(FeedbackPolarity::Warning)
            .with_salience(salience)
    }

    fn new(lesson_id: LessonId, input_kind: TeacherInputKind) -> Self {
        Self {
            lesson_id,
            input_kind,
            token_id: None,
            gesture_id: None,
            object_entity: None,
            feedback: None,
            salience: NormalizedScalar(0.5),
            confidence: Confidence(0.5),
        }
    }

    const fn with_token_id(mut self, token_id: u32) -> Self {
        self.token_id = Some(token_id);
        self
    }

    const fn with_gesture_id(mut self, gesture_id: u32) -> Self {
        self.gesture_id = Some(gesture_id);
        self
    }

    const fn with_object_entity(mut self, object_entity: WorldEntityId) -> Self {
        self.object_entity = Some(object_entity);
        self
    }

    const fn with_feedback(mut self, feedback: FeedbackPolarity) -> Self {
        self.feedback = Some(feedback);
        self
    }

    const fn with_salience(mut self, salience: NormalizedScalar) -> Self {
        self.salience = salience;
        self
    }

    const fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LessonPlan {
    pub lesson_id: LessonId,
    pub acts: Vec<TeacherAct>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannerVisibleState {
    pub developmental_stage_raw: u8,
    pub observable_success: bool,
    pub coarse_homeostatic_stress: NormalizedScalar,
    pub uncertainty: NormalizedScalar,
}

pub trait TeacherPlanner {
    fn plan(
        &self,
        step: &CurriculumStep,
        visible: PlannerVisibleState,
    ) -> Result<LessonPlan, ScaffoldContractError>;
}

/// Production planner for authored curricula. It can select bounded acts but
/// cannot mint learner-visible events; only the embodied actor can do that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumTeacherPlanner {
    max_acts_per_lesson: usize,
}

impl CurriculumTeacherPlanner {
    pub const fn bounded_default() -> Self {
        Self {
            max_acts_per_lesson: 16,
        }
    }
}

impl TeacherPlanner for CurriculumTeacherPlanner {
    fn plan(
        &self,
        step: &CurriculumStep,
        visible: PlannerVisibleState,
    ) -> Result<LessonPlan, ScaffoldContractError> {
        for value in [
            visible.coarse_homeostatic_stress.raw(),
            visible.uncertainty.raw(),
        ] {
            NormalizedScalar::new(value)?;
        }
        if step.prompt_cues.is_empty() || step.prompt_cues.len() > self.max_acts_per_lesson {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(LessonPlan {
            lesson_id: step.lesson_id,
            acts: step.prompt_cues.clone(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbodiedTeacherActor {
    teacher_entity: WorldEntityId,
}

impl EmbodiedTeacherActor {
    pub fn new(teacher_entity: WorldEntityId) -> Result<Self, ScaffoldContractError> {
        teacher_entity.validate()?;
        Ok(Self { teacher_entity })
    }

    pub const fn teacher_entity(&self) -> WorldEntityId {
        self.teacher_entity
    }

    pub fn enact(&self, act: TeacherAct) -> Result<TeacherPerceptualEvent, ScaffoldContractError> {
        let event = TeacherPerceptualEvent {
            schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
            lesson_id: act.lesson_id,
            input_kind: act.input_kind,
            channel: act.input_kind.channel(),
            token_id: act.token_id,
            gesture_id: act.gesture_id,
            object_entity: act.object_entity,
            feedback: act.feedback,
            salience: act.salience,
            confidence: act.confidence,
            teacher_entity: self.teacher_entity,
            actor_seal: self.teacher_entity.raw() ^ act.lesson_id.raw().rotate_left(17),
        };
        validate_event(&event)?;
        Ok(event)
    }

    pub fn enact_plan(
        &self,
        plan: &LessonPlan,
    ) -> Result<Vec<TeacherPerceptualEvent>, ScaffoldContractError> {
        if plan.acts.is_empty() || plan.acts.iter().any(|act| act.lesson_id != plan.lesson_id) {
            return Err(ScaffoldContractError::InvalidId);
        }
        plan.acts
            .iter()
            .copied()
            .map(|act| self.enact(act))
            .collect()
    }
}

impl TeacherPerceptualEvent {
    pub const fn channel(&self) -> TeacherPerceptionChannel {
        self.channel
    }

    pub const fn hidden_vector_injection_allowed(&self) -> bool {
        false
    }

    pub const fn direct_motor_bypass(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherChannelContract {
    pub schema_version: u16,
    pub channels: Vec<TeacherPerceptionChannel>,
    pub input_kinds: Vec<TeacherInputKind>,
    pub hidden_vector_injection_allowed: bool,
    pub direct_motor_bypass_allowed: bool,
}

impl TeacherChannelContract {
    pub fn grounded_default() -> Self {
        Self {
            schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
            channels: TeacherPerceptionChannel::ALL.to_vec(),
            input_kinds: TeacherInputKind::PERCEPTION_ONLY.to_vec(),
            hidden_vector_injection_allowed: false,
            direct_motor_bypass_allowed: false,
        }
    }

    pub fn accepts_event(&self, event: &TeacherPerceptualEvent) -> bool {
        self.schema_version == event.schema_version
            && !self.hidden_vector_injection_allowed
            && !self.direct_motor_bypass_allowed
            && event.input_kind.is_perceptual()
            && self.input_kinds.contains(&event.input_kind)
            && self.channels.contains(&event.channel)
            && validate_event(event).is_ok()
    }
}

fn validate_event(event: &TeacherPerceptualEvent) -> Result<(), ScaffoldContractError> {
    event.lesson_id.raw().validate()?;
    NormalizedScalar::new(event.salience.raw())?;
    Confidence::new(event.confidence.raw())?;
    if let Some(token_id) = event.token_id {
        if token_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
    }
    if let Some(gesture_id) = event.gesture_id {
        if gesture_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
    }
    if let Some(entity) = event.object_entity {
        entity.validate()?;
    }
    event.teacher_entity.validate()?;
    if event.actor_seal != event.teacher_entity.raw() ^ event.lesson_id.raw().rotate_left(17) {
        return Err(ScaffoldContractError::InvalidId);
    }
    Ok(())
}

trait ValidateNonZero {
    fn validate(self) -> Result<(), ScaffoldContractError>;
}

impl ValidateNonZero for u64 {
    fn validate(self) -> Result<(), ScaffoldContractError> {
        if self == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(())
        }
    }
}
