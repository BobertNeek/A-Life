//! v0 scaffold: lobe-to-lobe routing masks for future sparse projection schemas.

use serde::{Deserialize, Serialize};

use crate::{LobeKind, LobeLayout, ScaffoldContractError, UpdateCadence};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectionType {
    FeedForward = 0,
    Feedback = 1,
    Recurrent = 2,
    Modulatory = 3,
    MotorProposal = 4,
    Homeostatic = 5,
    LateralInhibition = 6,
}

impl ProjectionType {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::FeedForward),
            1 => Ok(Self::Feedback),
            2 => Ok(Self::Recurrent),
            3 => Ok(Self::Modulatory),
            4 => Ok(Self::MotorProposal),
            5 => Ok(Self::Homeostatic),
            6 => Ok(Self::LateralInhibition),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActiveTilePolicy {
    EssentialReservation = 0,
    SalienceGated = 1,
    Decimated = 2,
    SleepQueued = 3,
}

impl ActiveTilePolicy {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::EssentialReservation),
            1 => Ok(Self::SalienceGated),
            2 => Ok(Self::Decimated),
            3 => Ok(Self::SleepQueued),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiologicalPriority {
    Essential = 0,
    High = 1,
    Normal = 2,
    NonEssential = 3,
}

impl BiologicalPriority {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Essential),
            1 => Ok(Self::High),
            2 => Ok(Self::Normal),
            3 => Ok(Self::NonEssential),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoutingMask {
    pub source_lobe: LobeKind,
    pub target_lobe: LobeKind,
    pub projection_type: ProjectionType,
    pub active_tile_policy: ActiveTilePolicy,
    pub update_cadence: UpdateCadence,
    pub priority: BiologicalPriority,
}

impl RoutingMask {
    pub const fn new(
        source_lobe: LobeKind,
        target_lobe: LobeKind,
        projection_type: ProjectionType,
        active_tile_policy: ActiveTilePolicy,
        update_cadence: UpdateCadence,
        priority: BiologicalPriority,
    ) -> Self {
        Self {
            source_lobe,
            target_lobe,
            projection_type,
            active_tile_policy,
            update_cadence,
            priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingMatrix {
    routes: Vec<RoutingMask>,
}

impl RoutingMatrix {
    pub fn canonical_for_layout(layout: &LobeLayout) -> Self {
        let routes = CANONICAL_ROUTING_MASKS
            .iter()
            .copied()
            .filter(|mask| {
                layout
                    .region(mask.source_lobe)
                    .is_some_and(|region| region.enabled)
                    && layout
                        .region(mask.target_lobe)
                        .is_some_and(|region| region.enabled)
            })
            .collect();
        Self { routes }
    }

    pub fn from_masks(routes: Vec<RoutingMask>) -> Self {
        Self { routes }
    }

    pub fn masks(&self) -> &[RoutingMask] {
        &self.routes
    }

    pub fn push(&mut self, mask: RoutingMask) {
        self.routes.push(mask);
    }

    pub fn route(&self, source_lobe: LobeKind, target_lobe: LobeKind) -> Option<&RoutingMask> {
        self.routes
            .iter()
            .find(|mask| mask.source_lobe == source_lobe && mask.target_lobe == target_lobe)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RoutingMask> {
        self.routes.iter()
    }

    pub fn validate_for_layout(&self, layout: &LobeLayout) -> Result<(), ScaffoldContractError> {
        for (index, mask) in self.routes.iter().enumerate() {
            let source_enabled = layout
                .region(mask.source_lobe)
                .is_some_and(|region| region.enabled);
            let target_enabled = layout
                .region(mask.target_lobe)
                .is_some_and(|region| region.enabled);
            if !source_enabled || !target_enabled {
                return Err(ScaffoldContractError::RoutingReferencesDisabledLobe);
            }

            if self.routes[index + 1..].iter().any(|other| {
                other.source_lobe == mask.source_lobe
                    && other.target_lobe == mask.target_lobe
                    && other.projection_type == mask.projection_type
            }) {
                return Err(ScaffoldContractError::RoutingDuplicateMask);
            }
        }

        Ok(())
    }
}

const CANONICAL_ROUTING_MASKS: &[RoutingMask] = &[
    RoutingMask::new(
        LobeKind::SensoryGrounding,
        LobeKind::CoreAssociation,
        ProjectionType::FeedForward,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::MetabolicDrive,
        LobeKind::HomeostaticRegulation,
        ProjectionType::Homeostatic,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::MetabolicDrive,
        LobeKind::CoreAssociation,
        ProjectionType::Modulatory,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::AuditorySpeech,
        LobeKind::LexiconConcept,
        ProjectionType::FeedForward,
        ActiveTilePolicy::SalienceGated,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::High,
    ),
    RoutingMask::new(
        LobeKind::GlyphVision,
        LobeKind::LexiconConcept,
        ProjectionType::FeedForward,
        ActiveTilePolicy::SalienceGated,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::High,
    ),
    RoutingMask::new(
        LobeKind::LexiconConcept,
        LobeKind::CoreAssociation,
        ProjectionType::Modulatory,
        ActiveTilePolicy::SalienceGated,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::Normal,
    ),
    RoutingMask::new(
        LobeKind::CoreAssociation,
        LobeKind::EpisodicMemory,
        ProjectionType::FeedForward,
        ActiveTilePolicy::SalienceGated,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::NonEssential,
    ),
    RoutingMask::new(
        LobeKind::EpisodicMemory,
        LobeKind::CoreAssociation,
        ProjectionType::Feedback,
        ActiveTilePolicy::SalienceGated,
        UpdateCadence::Hot5To15Hz,
        BiologicalPriority::NonEssential,
    ),
    RoutingMask::new(
        LobeKind::CoreAssociation,
        LobeKind::WorkingMemory,
        ProjectionType::Recurrent,
        ActiveTilePolicy::Decimated,
        UpdateCadence::Hot15To60Hz,
        BiologicalPriority::High,
    ),
    RoutingMask::new(
        LobeKind::WorkingMemory,
        LobeKind::MotorArbitration,
        ProjectionType::MotorProposal,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::CoreAssociation,
        LobeKind::MotorArbitration,
        ProjectionType::MotorProposal,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::MotorArbitration,
        LobeKind::MotorArbitration,
        ProjectionType::LateralInhibition,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot60Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::MotorArbitration,
        LobeKind::HomeostaticRegulation,
        ProjectionType::Feedback,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::MetabolicDrive,
        ProjectionType::Homeostatic,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
    RoutingMask::new(
        LobeKind::HomeostaticRegulation,
        LobeKind::CoreAssociation,
        ProjectionType::Modulatory,
        ActiveTilePolicy::EssentialReservation,
        UpdateCadence::Hot10To30Hz,
        BiologicalPriority::Essential,
    ),
];
