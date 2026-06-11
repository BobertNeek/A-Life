//! v0 scaffold: shared validation traits and helpers.

use crate::{require_current_version, ScaffoldContractError, SchemaKind};

pub trait Validate {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validated<T>(T);

impl<T> Validated<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

impl<T: Validate> Validated<T> {
    pub fn try_new(value: T) -> Result<Self, ScaffoldContractError> {
        value.validate_contract()?;
        Ok(Self(value))
    }
}

pub fn ensure_current_version(kind: SchemaKind, actual: u16) -> Result<(), ScaffoldContractError> {
    require_current_version(kind, actual)
}
