//! v0 scaffold: scalable brain class registry and topology contracts.

use serde::{Deserialize, Serialize};

use crate::{
    require_current_version, BrainClassId, LobeKind, LobeLayout, RoutingMatrix,
    ScaffoldContractError, SchemaKind, SchemaVersions,
};

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
pub struct BrainComputeBudget {
    pub max_active_synapses: u32,
    pub max_active_tiles: u32,
    pub essential_lobes: Vec<LobeKind>,
    pub max_replay_events: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainClassSpec {
    pub id: BrainClassId,
    pub tier: BrainScaleTier,
    pub neuron_count: u32,
    pub microtile_edge: u32,
    pub supertile_edge: u32,
    pub compute_budget: BrainComputeBudget,
    pub max_active_microtiles: u32,
    pub max_active_synapses: u32,
    pub active_loop_resizing_allowed: bool,
    pub motor_logical_nodes: u32,
    pub motor_physical_stride: u32,
    pub routing_schema_version: u16,
    pub lobe_layout: LobeLayout,
    pub routing_matrix: RoutingMatrix,
}

impl BrainClassSpec {
    pub fn for_tier(tier: BrainScaleTier) -> Self {
        Self::try_for_tier(tier).expect("ResearchCustom requires BrainClassSpec::research_custom")
    }

    pub fn try_for_tier(tier: BrainScaleTier) -> Result<Self, ScaffoldContractError> {
        let neuron_count = tier
            .neuron_count()
            .ok_or(ScaffoldContractError::MissingCanonicalNeuronCount)?;
        let lobe_layout = LobeLayout::reference_for_neuron_count(neuron_count)?;
        Self::from_parts(
            tier.default_class_id(),
            tier,
            neuron_count,
            BrainComputeBudget::for_tier(tier, neuron_count),
            lobe_layout,
        )
    }

    pub fn research_custom(
        id: BrainClassId,
        neuron_count: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let lobe_layout = LobeLayout::reference_for_neuron_count(neuron_count)?;
        let spec = Self::from_parts(
            id,
            BrainScaleTier::ResearchCustom,
            neuron_count,
            BrainComputeBudget::for_custom(neuron_count),
            lobe_layout,
        )?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        if self.neuron_count < 512 {
            return Err(ScaffoldContractError::BrainClassTooSmall);
        }
        if self.tier.is_near_term_gpu_class() && !self.neuron_count.is_multiple_of(128) {
            return Err(ScaffoldContractError::BrainClassAlignment);
        }
        if self.microtile_edge != 16 || self.supertile_edge != 128 {
            return Err(ScaffoldContractError::BrainClassAlignment);
        }
        if self.max_active_synapses != self.compute_budget.max_active_synapses
            || self.max_active_microtiles != self.compute_budget.max_active_tiles
        {
            return Err(ScaffoldContractError::BrainClassAlignment);
        }

        self.lobe_layout
            .validate_for_neuron_count(self.neuron_count)?;
        self.routing_matrix.validate_for_layout(&self.lobe_layout)?;

        let motor_region = self
            .lobe_layout
            .region(LobeKind::MotorArbitration)
            .ok_or(ScaffoldContractError::LobeRangeCoverage)?;
        if !motor_region.enabled
            || self.motor_logical_nodes != motor_region.len
            || self.motor_logical_nodes == 0
            || !self.motor_logical_nodes.is_multiple_of(16)
            || self.motor_physical_stride < self.motor_logical_nodes
            || !self.motor_physical_stride.is_multiple_of(16)
        {
            return Err(ScaffoldContractError::LobeAlignment);
        }

        require_current_version(SchemaKind::NeuralProjection, self.routing_schema_version)?;
        Ok(())
    }

    pub fn lobe_by_neuron_index(&self, neuron_index: u32) -> Option<&crate::LobeRegion> {
        self.lobe_layout.lobe_by_neuron_index(neuron_index)
    }

    pub fn lobe_regions(&self) -> impl Iterator<Item = &crate::LobeRegion> {
        self.lobe_layout.iter_regions()
    }

    pub fn routing_masks(&self) -> &[crate::RoutingMask] {
        self.routing_matrix.masks()
    }

    fn from_parts(
        id: BrainClassId,
        tier: BrainScaleTier,
        neuron_count: u32,
        compute_budget: BrainComputeBudget,
        lobe_layout: LobeLayout,
    ) -> Result<Self, ScaffoldContractError> {
        let motor_logical_nodes = lobe_layout
            .region(LobeKind::MotorArbitration)
            .ok_or(ScaffoldContractError::LobeRangeCoverage)?
            .len;
        let routing_matrix = RoutingMatrix::canonical_for_layout(&lobe_layout);
        let spec = Self {
            id,
            tier,
            neuron_count,
            microtile_edge: 16,
            supertile_edge: 128,
            max_active_microtiles: compute_budget.max_active_tiles,
            max_active_synapses: compute_budget.max_active_synapses,
            active_loop_resizing_allowed: false,
            motor_logical_nodes,
            motor_physical_stride: padded_motor_stride(motor_logical_nodes),
            routing_schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
            compute_budget,
            lobe_layout,
            routing_matrix,
        };
        spec.validate()?;
        Ok(spec)
    }
}

impl BrainComputeBudget {
    pub fn for_tier(tier: BrainScaleTier, neuron_count: u32) -> Self {
        let (max_active_synapses, max_active_tiles, essential_lobes) = match tier {
            BrainScaleTier::Nano512 => (
                8_192,
                64,
                vec![
                    LobeKind::MetabolicDrive,
                    LobeKind::SensoryGrounding,
                    LobeKind::MotorArbitration,
                ],
            ),
            BrainScaleTier::Small1024 => (
                16_384,
                128,
                vec![
                    LobeKind::MetabolicDrive,
                    LobeKind::SensoryGrounding,
                    LobeKind::MotorArbitration,
                    LobeKind::CoreAssociation,
                ],
            ),
            BrainScaleTier::Standard2048 => (
                32_768,
                192,
                vec![
                    LobeKind::MetabolicDrive,
                    LobeKind::SensoryGrounding,
                    LobeKind::MotorArbitration,
                    LobeKind::CoreAssociation,
                    LobeKind::EpisodicMemory,
                ],
            ),
            BrainScaleTier::Large4096 => (
                65_536,
                384,
                vec![
                    LobeKind::MetabolicDrive,
                    LobeKind::SensoryGrounding,
                    LobeKind::MotorArbitration,
                    LobeKind::CoreAssociation,
                    LobeKind::EpisodicMemory,
                    LobeKind::WorkingMemory,
                ],
            ),
            BrainScaleTier::Cognitive32768
            | BrainScaleTier::Student131k
            | BrainScaleTier::Ascended1M
            | BrainScaleTier::Ascended5M => (
                neuron_count.saturating_mul(16),
                (neuron_count / 32).saturating_mul(3).max(384),
                vec![
                    LobeKind::MetabolicDrive,
                    LobeKind::SensoryGrounding,
                    LobeKind::MotorArbitration,
                    LobeKind::CoreAssociation,
                    LobeKind::EpisodicMemory,
                    LobeKind::WorkingMemory,
                ],
            ),
            BrainScaleTier::ResearchCustom => {
                return Self::for_custom(neuron_count);
            }
        };

        Self {
            max_active_synapses,
            max_active_tiles,
            essential_lobes,
            max_replay_events: replay_events_for_neuron_count(neuron_count),
        }
    }

    pub fn for_custom(neuron_count: u32) -> Self {
        Self {
            max_active_synapses: neuron_count.saturating_mul(16),
            max_active_tiles: (neuron_count / 16).max(1),
            essential_lobes: vec![
                LobeKind::MetabolicDrive,
                LobeKind::SensoryGrounding,
                LobeKind::MotorArbitration,
                LobeKind::HomeostaticRegulation,
            ],
            max_replay_events: replay_events_for_neuron_count(neuron_count),
        }
    }
}

pub struct BrainClassRegistry;

/// Compatibility boundary from legacy named tiers to stable capacity IDs.
pub struct LegacyBrainClassAdapter;

impl LegacyBrainClassAdapter {
    pub const fn capacity_id_for_tier(tier: BrainScaleTier) -> BrainClassId {
        tier.default_class_id()
    }
}

impl BrainClassRegistry {
    const CANONICAL_TIERS: [BrainScaleTier; 8] = [
        BrainScaleTier::Nano512,
        BrainScaleTier::Small1024,
        BrainScaleTier::Standard2048,
        BrainScaleTier::Large4096,
        BrainScaleTier::Cognitive32768,
        BrainScaleTier::Student131k,
        BrainScaleTier::Ascended1M,
        BrainScaleTier::Ascended5M,
    ];

    pub const fn canonical_tiers() -> &'static [BrainScaleTier; 8] {
        &Self::CANONICAL_TIERS
    }

    pub fn canonical_specs() -> Vec<BrainClassSpec> {
        Self::CANONICAL_TIERS
            .iter()
            .copied()
            .map(BrainClassSpec::for_tier)
            .collect()
    }

    pub fn spec_for_tier(tier: BrainScaleTier) -> Result<BrainClassSpec, ScaffoldContractError> {
        BrainClassSpec::try_for_tier(tier)
    }

    pub fn spec_for_id(id: BrainClassId) -> Option<BrainClassSpec> {
        Self::CANONICAL_TIERS
            .iter()
            .copied()
            .find(|tier| tier.default_class_id() == id)
            .map(BrainClassSpec::for_tier)
    }
}

fn padded_motor_stride(logical_nodes: u32) -> u32 {
    logical_nodes
        .checked_next_power_of_two()
        .unwrap_or_else(|| logical_nodes.div_ceil(16) * 16)
        .max(16)
}

fn replay_events_for_neuron_count(neuron_count: u32) -> u32 {
    match neuron_count {
        512 => 32,
        1024 => 64,
        2048 => 128,
        4096 => 256,
        _ => (neuron_count / 16).clamp(256, 16_384),
    }
}
