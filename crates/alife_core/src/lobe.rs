//! v0 scaffold: aligned lobe layout contracts, not runtime allocation.

use serde::{Deserialize, Serialize};

use crate::ScaffoldContractError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LobeKind {
    SensoryGrounding,
    MetabolicDrive,
    AuditorySpeech,
    GlyphVision,
    LexiconConcept,
    CoreAssociation,
    EpisodicMemory,
    WorkingMemory,
    MotorArbitration,
    HomeostaticRegulation,
}

impl LobeKind {
    pub const ALL: [LobeKind; 10] = [
        LobeKind::SensoryGrounding,
        LobeKind::MetabolicDrive,
        LobeKind::AuditorySpeech,
        LobeKind::GlyphVision,
        LobeKind::LexiconConcept,
        LobeKind::CoreAssociation,
        LobeKind::EpisodicMemory,
        LobeKind::WorkingMemory,
        LobeKind::MotorArbitration,
        LobeKind::HomeostaticRegulation,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeRegion {
    pub kind: LobeKind,
    pub start: u32,
    pub len: u32,
    pub enabled: bool,
}

impl LobeRegion {
    pub const fn end(self) -> u32 {
        self.start + self.len
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeLayout {
    pub regions: Vec<LobeRegion>,
}

impl LobeLayout {
    pub fn reference_for_neuron_count(neuron_count: u32) -> Result<Self, ScaffoldContractError> {
        Self::build(neuron_count, None)
    }

    pub fn with_disabled_lobe(
        neuron_count: u32,
        disabled: LobeKind,
    ) -> Result<Self, ScaffoldContractError> {
        Self::build(neuron_count, Some(disabled))
    }

    pub fn total_neurons(&self) -> u32 {
        self.regions.iter().map(|region| region.len).sum()
    }

    pub fn contains_lobe(&self, kind: LobeKind) -> bool {
        self.region(kind).is_some()
    }

    pub fn region(&self, kind: LobeKind) -> Option<&LobeRegion> {
        self.regions.iter().find(|region| region.kind == kind)
    }

    pub fn regions_are_aligned(&self, alignment: u32) -> bool {
        self.regions
            .iter()
            .all(|region| region.start % alignment == 0 && region.len % alignment == 0)
    }

    pub fn validate_for_neuron_count(
        &self,
        neuron_count: u32,
    ) -> Result<(), ScaffoldContractError> {
        if self.total_neurons() != neuron_count {
            return Err(ScaffoldContractError::LobeTotalMismatch);
        }
        if !self.regions_are_aligned(16) {
            return Err(ScaffoldContractError::LobeAlignment);
        }
        Ok(())
    }

    fn build(neuron_count: u32, disabled: Option<LobeKind>) -> Result<Self, ScaffoldContractError> {
        if neuron_count < 512 {
            return Err(ScaffoldContractError::BrainClassTooSmall);
        }
        if !neuron_count.is_multiple_of(16) {
            return Err(ScaffoldContractError::LobeAlignment);
        }

        let enabled_count = LobeKind::ALL
            .iter()
            .filter(|kind| Some(**kind) != disabled)
            .count() as u32;
        let mut regions = Vec::with_capacity(LobeKind::ALL.len());
        let mut start = 0;
        let mut remaining = neuron_count;
        let mut remaining_enabled = enabled_count;

        for kind in LobeKind::ALL {
            if Some(kind) == disabled {
                regions.push(LobeRegion {
                    kind,
                    start,
                    len: 0,
                    enabled: false,
                });
                continue;
            }

            let len = if remaining_enabled == 1 {
                remaining
            } else {
                ((remaining / remaining_enabled) / 16) * 16
            };

            regions.push(LobeRegion {
                kind,
                start,
                len,
                enabled: true,
            });
            start += len;
            remaining -= len;
            remaining_enabled -= 1;
        }

        let layout = Self { regions };
        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }
}
