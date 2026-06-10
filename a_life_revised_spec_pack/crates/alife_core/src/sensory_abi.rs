//! v0 scaffold: sensory channels and teacher perception boundaries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SensoryAbiVersion(pub u16);

impl SensoryAbiVersion {
    pub const CURRENT: Self = Self(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeacherPerceptionChannel {
    Hearing,
    Vision,
    Writing,
    Gesture,
    Object,
}

impl TeacherPerceptionChannel {
    pub const ALL: [TeacherPerceptionChannel; 5] = [
        TeacherPerceptionChannel::Hearing,
        TeacherPerceptionChannel::Vision,
        TeacherPerceptionChannel::Writing,
        TeacherPerceptionChannel::Gesture,
        TeacherPerceptionChannel::Object,
    ];
}
