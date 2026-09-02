#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlaybackState {
    Paused,
    Running,
    ShutdownRequested,
}

impl RuntimePlaybackState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paused => "paused",
            Self::Running => "running",
            Self::ShutdownRequested => "shutdown-requested",
        }
    }
}
