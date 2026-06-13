//! v0 scaffold: teacher roles and perception-only event contracts.

use alife_core::{
    Confidence, NormalizedScalar, ScaffoldContractError, SchemaVersions, TeacherPerceptionChannel,
    WorldEntityId,
};

use crate::LessonId;

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
    VisibleReward,
    VisiblePunishment,
}

impl TeacherInputKind {
    pub const PERCEPTION_ONLY: [Self; 6] = [
        Self::SpokenToken,
        Self::Gesture,
        Self::ObjectHighlight,
        Self::SocialFeedback,
        Self::VisibleReward,
        Self::VisiblePunishment,
    ];

    pub const fn is_perceptual(&self) -> bool {
        matches!(
            self,
            Self::SpokenToken
                | Self::Gesture
                | Self::ObjectHighlight
                | Self::SocialFeedback
                | Self::VisibleReward
                | Self::VisiblePunishment
        )
    }

    pub const fn channel(self) -> TeacherPerceptionChannel {
        match self {
            Self::SpokenToken => TeacherPerceptionChannel::Hearing,
            Self::Gesture => TeacherPerceptionChannel::Gesture,
            Self::ObjectHighlight => TeacherPerceptionChannel::Object,
            Self::SocialFeedback | Self::VisibleReward | Self::VisiblePunishment => {
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
}

impl TeacherPerceptualEvent {
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

    pub fn visible_reward(lesson_id: LessonId, salience: NormalizedScalar) -> Self {
        Self::new(lesson_id, TeacherInputKind::VisibleReward)
            .with_feedback(FeedbackPolarity::Praise)
            .with_salience(salience)
    }

    pub fn visible_punishment(lesson_id: LessonId, salience: NormalizedScalar) -> Self {
        Self::new(lesson_id, TeacherInputKind::VisiblePunishment)
            .with_feedback(FeedbackPolarity::Warning)
            .with_salience(salience)
    }

    pub const fn channel(&self) -> TeacherPerceptionChannel {
        self.channel
    }

    pub const fn hidden_vector_injection_allowed(&self) -> bool {
        false
    }

    pub const fn direct_motor_bypass(&self) -> bool {
        false
    }

    fn new(lesson_id: LessonId, input_kind: TeacherInputKind) -> Self {
        Self {
            schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
            lesson_id,
            input_kind,
            channel: input_kind.channel(),
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
