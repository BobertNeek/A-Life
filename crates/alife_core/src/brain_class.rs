//! v0 scaffold: scalable brain class contracts.

use serde::{Deserialize, Serialize};

use crate::{BrainClassId, LobeLayout, ScaffoldContractError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrainScaleTier {
    Nano512,
    Small1024,
    Standard2048,
    Large4096,
    Cognitive32768,
    Student131k,
    Ascended1M,
    Ascended5M,
    ResearchCustom,
}

impl BrainScaleTier {
    pub const fn neuron_count(self) -> Option<u32> {
        match self {
            BrainScaleTier::Nano512 => Some(512),
            BrainScaleTier::Small1024 => Some(1024),
            BrainScaleTier::Standard2048 => Some(2048),
            BrainScaleTier::Large4096 => Some(4096),
            BrainScaleTier::Cognitive32768 => Some(32_768),
            BrainScaleTier::Student131k => Some(131_072),
            BrainScaleTier::Ascended1M => Some(1_048_576),
            BrainScaleTier::Ascended5M => Some(5_242_880),
            BrainScaleTier::ResearchCustom => None,
        }
    }

    pub const fn default_class_id(self) -> BrainClassId {
        BrainClassId(match self {
            BrainScaleTier::Nano512 => 1,
            BrainScaleTier::Small1024 => 2,
            BrainScaleTier::Standard2048 => 3,
            BrainScaleTier::Large4096 => 4,
            BrainScaleTier::Cognitive32768 => 5,
            BrainScaleTier::Student131k => 6,
            BrainScaleTier::Ascended1M => 7,
            BrainScaleTier::Ascended5M => 8,
            BrainScaleTier::ResearchCustom => 65_535,
        })
    }

    pub const fn is_near_term_gpu_class(self) -> bool {
        matches!(
            self,
            BrainScaleTier::Nano512
                | BrainScaleTier::Small1024
                | BrainScaleTier::Standard2048
                | BrainScaleTier::Large4096
                | BrainScaleTier::Cognitive32768
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainClassSpec {
    pub id: BrainClassId,
    pub tier: BrainScaleTier,
    pub neuron_count: u32,
    pub microtile_edge: u32,
    pub supertile_edge: u32,
    pub max_active_microtiles: u32,
    pub max_active_synapses: u32,
    pub active_loop_resizing_allowed: bool,
    pub lobe_layout: LobeLayout,
}

impl BrainClassSpec {
    pub fn for_tier(tier: BrainScaleTier) -> Self {
        let neuron_count = tier
            .neuron_count()
            .expect("ResearchCustom requires BrainClassSpec::research_custom");
        Self {
            id: tier.default_class_id(),
            tier,
            neuron_count,
            microtile_edge: 16,
            supertile_edge: 128,
            max_active_microtiles: (neuron_count / 16).max(1),
            max_active_synapses: neuron_count.saturating_mul(32),
            active_loop_resizing_allowed: false,
            lobe_layout: LobeLayout::reference_for_neuron_count(neuron_count)
                .expect("canonical brain class layout must validate"),
        }
    }

    pub fn research_custom(
        id: BrainClassId,
        neuron_count: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let spec = Self {
            id,
            tier: BrainScaleTier::ResearchCustom,
            neuron_count,
            microtile_edge: 16,
            supertile_edge: 128,
            max_active_microtiles: (neuron_count / 16).max(1),
            max_active_synapses: neuron_count.saturating_mul(32),
            active_loop_resizing_allowed: false,
            lobe_layout: LobeLayout::reference_for_neuron_count(neuron_count)?,
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.neuron_count < 512 {
            return Err(ScaffoldContractError::BrainClassTooSmall);
        }
        if self.tier.is_near_term_gpu_class() && !self.neuron_count.is_multiple_of(128) {
            return Err(ScaffoldContractError::BrainClassAlignment);
        }
        self.lobe_layout
            .validate_for_neuron_count(self.neuron_count)?;
        Ok(())
    }
}
