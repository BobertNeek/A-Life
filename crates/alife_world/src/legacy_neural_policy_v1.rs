//! Deserialize-only policy migration for historical runtime-config records.

use alife_core::PolicyBackend;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LegacyBackendSelectionV1 {
    CpuReference,
    GpuStatic,
    GpuPlastic,
    GpuFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LegacyBackendConfigV1 {
    pub(crate) requested: LegacyBackendSelectionV1,
    #[serde(rename = "gpu_feature_enabled")]
    _gpu_feature_enabled: bool,
    #[serde(rename = "fallback_to_cpu")]
    _fallback_to_cpu: bool,
    #[serde(rename = "validation_required")]
    _validation_required: bool,
}

impl LegacyBackendConfigV1 {
    pub(crate) const fn migrate_policy(self) -> PolicyBackend {
        match self.requested {
            LegacyBackendSelectionV1::CpuReference => PolicyBackend::HeuristicBaseline,
            LegacyBackendSelectionV1::GpuStatic
            | LegacyBackendSelectionV1::GpuPlastic
            | LegacyBackendSelectionV1::GpuFull => PolicyBackend::NeuralClosedLoopGpu,
        }
    }
}
