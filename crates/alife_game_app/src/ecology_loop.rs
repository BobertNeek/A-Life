//! Terrain-zone resource and hazard indicators.

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct EcologyIndicator {
    pub zone_id: EcologyZoneId,
    pub label: String,
    pub terrain_kind: TerrainZoneKind,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl EcologyIndicator {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.zone_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.resource_bias)?;
        NormalizedScalar::new(self.hazard_pressure)?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.3}:{:.3}",
            self.zone_id.raw(),
            self.label,
            self.terrain_kind.label(),
            self.resource_bias,
            self.hazard_pressure
        )
    }
}
