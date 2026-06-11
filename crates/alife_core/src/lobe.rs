//! v0 scaffold: aligned lobe layout contracts, not runtime allocation.

use serde::{Deserialize, Serialize};

use crate::{LobeIndex, ScaffoldContractError};

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
    LanguageExpansion,
    MathQuantity,
    NarrativeHistory,
    SocialReasoning,
    SelfCriticUncertainty,
    PlanningDream,
    SpeechWritingMotor,
}

impl LobeKind {
    pub const CORE: [LobeKind; 10] = [
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

    pub const ALL: [LobeKind; 17] = [
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
        LobeKind::LanguageExpansion,
        LobeKind::MathQuantity,
        LobeKind::NarrativeHistory,
        LobeKind::SocialReasoning,
        LobeKind::SelfCriticUncertainty,
        LobeKind::PlanningDream,
        LobeKind::SpeechWritingMotor,
    ];

    pub const fn stable_id(self) -> LobeIndex {
        LobeIndex(match self {
            LobeKind::SensoryGrounding => 1,
            LobeKind::MetabolicDrive => 2,
            LobeKind::AuditorySpeech => 3,
            LobeKind::GlyphVision => 4,
            LobeKind::LexiconConcept => 5,
            LobeKind::CoreAssociation => 6,
            LobeKind::EpisodicMemory => 7,
            LobeKind::WorkingMemory => 8,
            LobeKind::MotorArbitration => 9,
            LobeKind::HomeostaticRegulation => 10,
            LobeKind::LanguageExpansion => 11,
            LobeKind::MathQuantity => 12,
            LobeKind::NarrativeHistory => 13,
            LobeKind::SocialReasoning => 14,
            LobeKind::SelfCriticUncertainty => 15,
            LobeKind::PlanningDream => 16,
            LobeKind::SpeechWritingMotor => 17,
        })
    }

    pub const fn purpose(self) -> &'static str {
        match self {
            LobeKind::SensoryGrounding => "grounded sensory affordance integration",
            LobeKind::MetabolicDrive => "drive and survival pressure encoding",
            LobeKind::AuditorySpeech => "hearing and speech perception",
            LobeKind::GlyphVision => "visible glyph and reading perception",
            LobeKind::LexiconConcept => "lexicon and concept binding",
            LobeKind::CoreAssociation => "cross-modal association and salience binding",
            LobeKind::EpisodicMemory => "episodic memory indexing and expectancy recall",
            LobeKind::WorkingMemory => "attention and short-horizon working state",
            LobeKind::MotorArbitration => "motor proposal competition and action staging",
            LobeKind::HomeostaticRegulation => "homeostatic regulation and safety feedback",
            LobeKind::LanguageExpansion => "future expanded language capacity",
            LobeKind::MathQuantity => "future quantity and math concepts",
            LobeKind::NarrativeHistory => "future narrative and history concepts",
            LobeKind::SocialReasoning => "future social reasoning capacity",
            LobeKind::SelfCriticUncertainty => "future self-critic and uncertainty monitoring",
            LobeKind::PlanningDream => "future planning and dream simulation",
            LobeKind::SpeechWritingMotor => "future speech and writing motor control",
        }
    }

    pub const fn default_update_cadence(self) -> UpdateCadence {
        match self {
            LobeKind::SensoryGrounding | LobeKind::MotorArbitration => UpdateCadence::Hot60Hz,
            LobeKind::MetabolicDrive | LobeKind::HomeostaticRegulation => {
                UpdateCadence::Hot10To30Hz
            }
            LobeKind::AuditorySpeech
            | LobeKind::GlyphVision
            | LobeKind::CoreAssociation
            | LobeKind::WorkingMemory => UpdateCadence::Hot15To60Hz,
            LobeKind::LexiconConcept | LobeKind::EpisodicMemory => UpdateCadence::Hot5To15Hz,
            LobeKind::LanguageExpansion
            | LobeKind::MathQuantity
            | LobeKind::NarrativeHistory
            | LobeKind::SocialReasoning
            | LobeKind::SelfCriticUncertainty
            | LobeKind::PlanningDream
            | LobeKind::SpeechWritingMotor => UpdateCadence::Disabled,
        }
    }

    pub const fn default_plasticity_policy(self) -> PlasticityPolicy {
        match self {
            LobeKind::SensoryGrounding
            | LobeKind::MetabolicDrive
            | LobeKind::MotorArbitration
            | LobeKind::HomeostaticRegulation => PlasticityPolicy::Modulated,
            LobeKind::CoreAssociation | LobeKind::WorkingMemory => PlasticityPolicy::FastOjaHebbian,
            LobeKind::LexiconConcept | LobeKind::EpisodicMemory => {
                PlasticityPolicy::DecimatedOjaHebbian
            }
            LobeKind::AuditorySpeech | LobeKind::GlyphVision => PlasticityPolicy::Modulated,
            LobeKind::LanguageExpansion
            | LobeKind::MathQuantity
            | LobeKind::NarrativeHistory
            | LobeKind::SocialReasoning
            | LobeKind::SelfCriticUncertainty
            | LobeKind::PlanningDream
            | LobeKind::SpeechWritingMotor => PlasticityPolicy::Disabled,
        }
    }

    pub const fn default_activation_policy(self) -> ActivationPolicy {
        match self {
            LobeKind::SensoryGrounding | LobeKind::AuditorySpeech | LobeKind::GlyphVision => {
                ActivationPolicy::SensoryInput
            }
            LobeKind::MetabolicDrive => ActivationPolicy::DriveState,
            LobeKind::LexiconConcept => ActivationPolicy::SparseAssociative,
            LobeKind::CoreAssociation => ActivationPolicy::SparseAssociative,
            LobeKind::EpisodicMemory => ActivationPolicy::EpisodicRecall,
            LobeKind::WorkingMemory => ActivationPolicy::WorkingAttention,
            LobeKind::MotorArbitration => ActivationPolicy::MotorCompetition,
            LobeKind::HomeostaticRegulation => ActivationPolicy::HomeostaticControl,
            LobeKind::LanguageExpansion
            | LobeKind::MathQuantity
            | LobeKind::NarrativeHistory
            | LobeKind::SocialReasoning
            | LobeKind::SelfCriticUncertainty
            | LobeKind::PlanningDream
            | LobeKind::SpeechWritingMotor => ActivationPolicy::Disabled,
        }
    }

    pub const fn default_essentiality(self) -> LobeEssentiality {
        match self {
            LobeKind::SensoryGrounding
            | LobeKind::MetabolicDrive
            | LobeKind::MotorArbitration
            | LobeKind::HomeostaticRegulation => LobeEssentiality::Essential,
            _ => LobeEssentiality::NonEssential,
        }
    }

    pub const fn default_throttle_priority(self) -> LobeThrottlePriority {
        match self {
            LobeKind::MetabolicDrive
            | LobeKind::SensoryGrounding
            | LobeKind::MotorArbitration
            | LobeKind::HomeostaticRegulation => LobeThrottlePriority::Critical,
            LobeKind::CoreAssociation | LobeKind::WorkingMemory => LobeThrottlePriority::High,
            LobeKind::AuditorySpeech | LobeKind::GlyphVision | LobeKind::LexiconConcept => {
                LobeThrottlePriority::Medium
            }
            LobeKind::EpisodicMemory => LobeThrottlePriority::Low,
            LobeKind::LanguageExpansion
            | LobeKind::MathQuantity
            | LobeKind::NarrativeHistory
            | LobeKind::SocialReasoning
            | LobeKind::SelfCriticUncertainty
            | LobeKind::PlanningDream
            | LobeKind::SpeechWritingMotor => LobeThrottlePriority::SleepOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpdateCadence {
    Hot60Hz,
    Hot15To60Hz,
    Hot10To30Hz,
    Hot5To15Hz,
    Hot1To5Hz,
    SleepOrOffline,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlasticityPolicy {
    Fixed,
    Modulated,
    FastOjaHebbian,
    DecimatedOjaHebbian,
    SleepConsolidationOnly,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivationPolicy {
    SensoryInput,
    DriveState,
    SparseAssociative,
    EpisodicRecall,
    WorkingAttention,
    MotorCompetition,
    HomeostaticControl,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LobeEssentiality {
    Essential,
    NonEssential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LobeThrottlePriority {
    Critical,
    High,
    Medium,
    Low,
    SleepOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeRegion {
    pub id: LobeIndex,
    pub kind: LobeKind,
    pub start: u32,
    pub len: u32,
    pub enabled: bool,
    pub update_cadence: UpdateCadence,
    pub plasticity_policy: PlasticityPolicy,
    pub activation_policy: ActivationPolicy,
    pub essentiality: LobeEssentiality,
    pub throttle_priority: LobeThrottlePriority,
}

impl LobeRegion {
    pub fn enabled(kind: LobeKind, start: u32, len: u32) -> Self {
        Self {
            id: kind.stable_id(),
            kind,
            start,
            len,
            enabled: true,
            update_cadence: kind.default_update_cadence(),
            plasticity_policy: kind.default_plasticity_policy(),
            activation_policy: kind.default_activation_policy(),
            essentiality: kind.default_essentiality(),
            throttle_priority: kind.default_throttle_priority(),
        }
    }

    pub fn disabled(kind: LobeKind, start: u32) -> Self {
        Self {
            id: kind.stable_id(),
            kind,
            start,
            len: 0,
            enabled: false,
            update_cadence: UpdateCadence::Disabled,
            plasticity_policy: PlasticityPolicy::Disabled,
            activation_policy: ActivationPolicy::Disabled,
            essentiality: LobeEssentiality::NonEssential,
            throttle_priority: LobeThrottlePriority::SleepOnly,
        }
    }

    pub const fn end(self) -> u32 {
        self.start + self.len
    }

    pub const fn end_exclusive(self) -> u32 {
        self.end()
    }

    pub const fn contains_neuron(self, neuron_index: u32) -> bool {
        self.enabled && self.start <= neuron_index && neuron_index < self.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeLayout {
    pub regions: Vec<LobeRegion>,
}

impl LobeLayout {
    pub fn reference_for_neuron_count(neuron_count: u32) -> Result<Self, ScaffoldContractError> {
        Self::build(neuron_count, None, LayoutMode::Reference)
    }

    pub fn with_disabled_lobe(
        neuron_count: u32,
        disabled: LobeKind,
    ) -> Result<Self, ScaffoldContractError> {
        Self::build(
            neuron_count,
            Some(disabled),
            LayoutMode::EqualSplitCompatibility,
        )
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

    pub fn lobe_by_neuron_index(&self, neuron_index: u32) -> Option<&LobeRegion> {
        self.enabled_regions()
            .find(|region| region.contains_neuron(neuron_index))
    }

    pub fn iter_regions(&self) -> impl Iterator<Item = &LobeRegion> {
        self.regions.iter()
    }

    pub fn enabled_regions(&self) -> impl Iterator<Item = &LobeRegion> {
        self.regions.iter().filter(|region| region.enabled)
    }

    pub fn routing_lobes(&self) -> impl Iterator<Item = LobeKind> + '_ {
        self.enabled_regions().map(|region| region.kind)
    }

    pub fn regions_are_aligned(&self, alignment: u32) -> bool {
        alignment != 0
            && self
                .regions
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

        let mut cursor = 0;
        for region in self.enabled_regions() {
            if region.len == 0 || region.start != cursor || region.end() > neuron_count {
                return Err(ScaffoldContractError::LobeRangeCoverage);
            }
            cursor = region.end();
        }
        if cursor != neuron_count {
            return Err(ScaffoldContractError::LobeRangeCoverage);
        }

        for kind in LobeKind::ALL {
            if self
                .regions
                .iter()
                .filter(|region| region.kind == kind)
                .count()
                != 1
            {
                return Err(ScaffoldContractError::LobeRangeCoverage);
            }
        }

        Ok(())
    }

    fn build(
        neuron_count: u32,
        disabled: Option<LobeKind>,
        mode: LayoutMode,
    ) -> Result<Self, ScaffoldContractError> {
        if neuron_count < 512 {
            return Err(ScaffoldContractError::BrainClassTooSmall);
        }
        if !neuron_count.is_multiple_of(16) {
            return Err(ScaffoldContractError::LobeAlignment);
        }

        let layout = match mode {
            LayoutMode::Reference => Self::reference_layout(neuron_count)?,
            LayoutMode::EqualSplitCompatibility => {
                Self::equal_split_layout(neuron_count, disabled)?
            }
        };

        if mode == LayoutMode::Reference {
            if let Some(disabled_kind) = disabled {
                let compact = Self::equal_split_layout(neuron_count, Some(disabled_kind))?;
                compact.validate_for_neuron_count(neuron_count)?;
                return Ok(compact);
            }
        }

        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }

    fn reference_layout(neuron_count: u32) -> Result<Self, ScaffoldContractError> {
        let lengths = reference_core_lengths(neuron_count)?;
        let regions = build_regions_from_lengths(neuron_count, &lengths);
        let layout = Self { regions };
        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }

    fn equal_split_layout(
        neuron_count: u32,
        disabled: Option<LobeKind>,
    ) -> Result<Self, ScaffoldContractError> {
        let enabled_count = LobeKind::CORE
            .iter()
            .filter(|kind| Some(**kind) != disabled)
            .count() as u32;
        let mut regions = Vec::with_capacity(LobeKind::ALL.len());
        let mut start = 0;
        let mut remaining = neuron_count;
        let mut remaining_enabled = enabled_count;

        for kind in LobeKind::CORE {
            if Some(kind) == disabled {
                regions.push(LobeRegion::disabled(kind, start));
                continue;
            }

            let len = if remaining_enabled == 1 {
                remaining
            } else {
                ((remaining / remaining_enabled) / 16) * 16
            };
            regions.push(LobeRegion::enabled(kind, start, len));
            start += len;
            remaining -= len;
            remaining_enabled -= 1;
        }
        for kind in future_lobes() {
            regions.push(LobeRegion::disabled(kind, neuron_count));
        }

        let layout = Self { regions };
        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Reference,
    EqualSplitCompatibility,
}

fn build_regions_from_lengths(neuron_count: u32, lengths: &[u32; 10]) -> Vec<LobeRegion> {
    let mut regions = Vec::with_capacity(LobeKind::ALL.len());
    let mut start = 0;
    for (kind, len) in LobeKind::CORE.into_iter().zip(lengths.iter().copied()) {
        regions.push(LobeRegion::enabled(kind, start, len));
        start += len;
    }
    for kind in future_lobes() {
        regions.push(LobeRegion::disabled(kind, neuron_count));
    }
    regions
}

fn future_lobes() -> [LobeKind; 7] {
    [
        LobeKind::LanguageExpansion,
        LobeKind::MathQuantity,
        LobeKind::NarrativeHistory,
        LobeKind::SocialReasoning,
        LobeKind::SelfCriticUncertainty,
        LobeKind::PlanningDream,
        LobeKind::SpeechWritingMotor,
    ]
}

fn reference_core_lengths(neuron_count: u32) -> Result<[u32; 10], ScaffoldContractError> {
    let lengths = match neuron_count {
        512 => [64, 32, 32, 32, 64, 112, 64, 32, 64, 16],
        1024 => [128, 64, 64, 64, 128, 224, 128, 64, 112, 48],
        n if n >= 2048 && n.is_multiple_of(2048) => {
            let scale = n / 2048;
            [
                256 * scale,
                128 * scale,
                128 * scale,
                128 * scale,
                256 * scale,
                448 * scale,
                256 * scale,
                128 * scale,
                224 * scale,
                96 * scale,
            ]
        }
        other => proportional_core_lengths(other)?,
    };

    debug_assert_eq!(lengths.iter().sum::<u32>(), neuron_count);
    Ok(lengths)
}

fn proportional_core_lengths(neuron_count: u32) -> Result<[u32; 10], ScaffoldContractError> {
    if neuron_count < 512 {
        return Err(ScaffoldContractError::BrainClassTooSmall);
    }
    if !neuron_count.is_multiple_of(16) {
        return Err(ScaffoldContractError::LobeAlignment);
    }

    let weights = [4_u64, 2, 2, 2, 4, 7, 4, 2, 4, 1];
    let weight_sum: u64 = weights.iter().sum();
    let mut lengths = [0; 10];
    let mut used = 0;
    for index in 0..weights.len() - 1 {
        let len = (((u64::from(neuron_count) * weights[index] / weight_sum) / 16) * 16) as u32;
        lengths[index] = len.max(16);
        used += lengths[index];
    }
    lengths[weights.len() - 1] = neuron_count.saturating_sub(used);
    if !lengths[weights.len() - 1].is_multiple_of(16) || lengths[weights.len() - 1] == 0 {
        return Err(ScaffoldContractError::LobeAlignment);
    }
    Ok(lengths)
}
