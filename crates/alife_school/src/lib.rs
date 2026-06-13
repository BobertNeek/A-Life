//! v0 scaffold: external in-world teacher contracts.
//!
//! The school crate may plan curriculum privately, but every creature-facing
//! signal is represented as ordinary perceptual or social evidence.

pub mod curriculum;
pub mod lesson_api;
pub mod runner;
pub mod teacher;
pub mod verifier;

pub use curriculum::{Curriculum, CurriculumStep, CurriculumStepKind, ExpectedObservation};
pub use lesson_api::{LessonId, LessonResponse, LessonResponseKind};
pub use runner::{HeadlessCurriculumRunner, LessonDispatch};
pub use teacher::{
    FeedbackPolarity, TeacherChannelContract, TeacherInputKind, TeacherPerceptualEvent,
    TeacherRole, TEACHER_SCHOOL_SCHEMA_VERSION,
};
pub use verifier::{
    LessonVerification, LessonVerifier, PatchLogLessonVerifier, SchoolEvidence, TopologySummary,
    VerifierCheck,
};
