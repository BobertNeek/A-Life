//! v0 scaffold: developer tooling contracts.

pub mod benchmark;
pub mod p30_bundle;
pub mod p30_cluster;
pub mod p30_markers;
pub mod p30_summary;
pub mod p31_offline_tools;
pub mod p32_weights;
pub mod p33_evolution;
pub mod p35_playground;

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
