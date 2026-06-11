//! v0 scaffold: action ABI versioning.

use serde::{Deserialize, Serialize};

use crate::SchemaVersions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionAbiVersion(pub u16);

impl ActionAbiVersion {
    pub const CURRENT: Self = Self(SchemaVersions::CURRENT.action_abi.0);
}
