//! v0 scaffold: lineage export and migration manifest contracts.

use serde::{Deserialize, Serialize};

use crate::{BrainClassId, GenomeId, LineageId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageExportManifest {
    pub abi_version: u16,
    pub lineage_id: LineageId,
    pub founder_genome_id: GenomeId,
    pub source_brain_class_id: BrainClassId,
    pub target_brain_class_id: Option<BrainClassId>,
    pub exported_at_tick: u64,
}

impl LineageExportManifest {
    pub const ABI_VERSION: u16 = 1;
}
