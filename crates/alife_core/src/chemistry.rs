//! v0 scaffold: endocrine and drive modulation contracts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndocrineProfile {
    pub dopamine_baseline: f32,
    pub serotonin_baseline: f32,
    pub cortisol_baseline: f32,
    pub oxytocin_baseline: f32,
    pub adrenaline_baseline: f32,
    pub acetylcholine_baseline: f32,
}

impl EndocrineProfile {
    pub const fn baseline() -> Self {
        Self {
            dopamine_baseline: 1.0,
            serotonin_baseline: 1.0,
            cortisol_baseline: 0.2,
            oxytocin_baseline: 0.5,
            adrenaline_baseline: 0.2,
            acetylcholine_baseline: 1.0,
        }
    }

    pub const fn modulator_count(&self) -> usize {
        6
    }
}
