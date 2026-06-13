//! v0 scaffold: lesson identity and response metadata contracts.

use alife_core::{
    ScaffoldContractError, TeacherLessonMetadata, TeacherLessonResponseChannel, WorldEntityId,
};

use crate::TEACHER_SCHOOL_SCHEMA_VERSION;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LessonId(u64);

impl LessonId {
    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn new(raw: u64) -> Result<Self, ScaffoldContractError> {
        if raw == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(Self(raw))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonResponseKind {
    CreatureApproached,
    CreatureGrabbed,
    CreatureVocalized,
    CreatureInspected,
    CreatureAcceptedFood,
    CreatureAvoidedHazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LessonResponse {
    pub schema_version: u16,
    pub lesson_id: LessonId,
    pub kind: LessonResponseKind,
    pub response_channel: TeacherLessonResponseChannel,
    pub teacher_entity: Option<WorldEntityId>,
}

impl LessonResponse {
    pub const fn new(
        lesson_id: LessonId,
        kind: LessonResponseKind,
        response_channel: TeacherLessonResponseChannel,
    ) -> Self {
        Self {
            schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
            lesson_id,
            kind,
            response_channel,
            teacher_entity: None,
        }
    }

    pub const fn with_teacher_entity(mut self, teacher_entity: WorldEntityId) -> Self {
        self.teacher_entity = Some(teacher_entity);
        self
    }

    pub fn to_action_metadata(self) -> Result<TeacherLessonMetadata, ScaffoldContractError> {
        if self.schema_version != TEACHER_SCHOOL_SCHEMA_VERSION {
            return Err(ScaffoldContractError::IncompatibleAbi {
                kind: alife_core::SchemaKind::TeacherSchool,
                expected: TEACHER_SCHOOL_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        let metadata = TeacherLessonMetadata {
            teacher_entity: self.teacher_entity,
            lesson_id: self.lesson_id.raw(),
            response_channel: self.response_channel,
        };
        metadata.validate()
    }
}
