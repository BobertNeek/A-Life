//! A registry-owned record stays validated only until its next mutable borrow.
//!
//! Keep the fields private to this child module: even the registry and its tests
//! must pass through `DerefMut` to modify the canonical record. Shared access can
//! only fill existing derived caches; it cannot change organism state.

use std::{
    ops::{Deref, DerefMut},
    sync::OnceLock,
};

use super::{ScaffoldContractError, WorldOrganismRecord};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub(super) struct RegisteredRecord {
    record: WorldOrganismRecord,
    validated: OnceLock<()>,
}

impl RegisteredRecord {
    pub(super) fn new(record: WorldOrganismRecord) -> Self {
        Self {
            record,
            validated: OnceLock::new(),
        }
    }

    pub(super) fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.validated.get().is_none() {
            self.record.validate_contract()?;
            // Concurrent readers validate the same immutable record.
            let _ = self.validated.set(());
        }
        Ok(())
    }

    pub(super) fn into_inner(self) -> WorldOrganismRecord {
        self.record
    }

    #[cfg(test)]
    pub(super) fn is_validated(&self) -> bool {
        self.validated.get().is_some()
    }
}

impl Deref for RegisteredRecord {
    type Target = WorldOrganismRecord;

    fn deref(&self) -> &Self::Target {
        &self.record
    }
}

impl DerefMut for RegisteredRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.validated.take();
        &mut self.record
    }
}

impl Serialize for RegisteredRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.record.serialize(serializer)
    }
}
