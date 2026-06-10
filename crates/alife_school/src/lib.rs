//! v0 scaffold: external in-world teacher contracts.

use alife_core::TeacherPerceptionChannel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherRole {
    Tutor,
    Examiner,
    Critic,
    CurriculumPlanner,
    Verifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherChannelContract {
    pub channels: Vec<TeacherPerceptionChannel>,
    pub hidden_vector_injection_allowed: bool,
}

impl TeacherChannelContract {
    pub fn grounded_default() -> Self {
        Self {
            channels: TeacherPerceptionChannel::ALL.to_vec(),
            hidden_vector_injection_allowed: false,
        }
    }
}
