//! v0 scaffold: lineage export and migration manifest contracts.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, BrainClassId, GenomeId, LineageId, SchemaKind, SchemaVersions, Validate,
};

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
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.lineage_export.0;
}

impl Validate for LineageExportManifest {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        ensure_current_version(SchemaKind::LineageExport, self.abi_version)?;
        self.lineage_id.validate()?;
        self.founder_genome_id.validate()?;
        self.source_brain_class_id.validate()?;
        if let Some(target) = self.target_brain_class_id {
            target.validate()?;
        }
        Ok(())
    }
}
