//! v0 scaffold: lobe-to-lobe routing masks for future sparse projection schemas.

use serde::{Deserialize, Serialize};

use crate::{LobeKind, LobeLayout, ScaffoldContractError, UpdateCadence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectionType {
    FeedForward,
    Feedback,
    Recurrent,
    Modulatory,
    MotorProposal,
    Homeostatic,
    LateralInhibition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActiveTilePolicy {
    EssentialReservation,
    SalienceGated,
    Decimated,
    SleepQueued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiologicalPriority {
    Essential,
    High,
    Normal,
    NonEssential,
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
