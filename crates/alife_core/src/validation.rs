//! Shared validation traits and helpers.

use crate::{require_current_version, ScaffoldContractError, SchemaKind};

pub trait Validate {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

pub fn ensure_current_version(kind: SchemaKind, actual: u16) -> Result<(), ScaffoldContractError> {
    require_current_version(kind, actual)
}
