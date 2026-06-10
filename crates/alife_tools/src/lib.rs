//! v0 scaffold: developer tooling contracts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolingManifest {
    pub graphify_required_for_cargo_build: bool,
    pub dox_agents_required: bool,
}

impl ToolingManifest {
    pub const CURRENT: Self = Self {
        graphify_required_for_cargo_build: false,
        dox_agents_required: true,
    };
}
