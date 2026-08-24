//! Aligned lobe layout contracts, independent of runtime allocation.

use serde::{Deserialize, Serialize};

use crate::{LobeIndex, ScaffoldContractError};

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LobeKind {
    PerceptualIntegration = 1,
    InteroceptiveMotivational = 2,
    MultimodalAssociation = 3,
    TemporalPredictive = 4,
    WorkingContextExecutive = 5,
    MemoryInterface = 6,
    ActionPlanning = 7,
    SocialCommunication = 8,
    FlexibleReserve = 9,
}

impl LobeKind {
    pub const CORE: [LobeKind; 9] = Self::ALL;

    pub const ALL: [LobeKind; 9] = [
        LobeKind::PerceptualIntegration,
        LobeKind::InteroceptiveMotivational,
        LobeKind::MultimodalAssociation,
        LobeKind::TemporalPredictive,
        LobeKind::WorkingContextExecutive,
        LobeKind::MemoryInterface,
        LobeKind::ActionPlanning,
        LobeKind::SocialCommunication,
        LobeKind::FlexibleReserve,
    ];

    // Temporary source compatibility for pre-v2 callers. Durable v1 IDs use
    // `LegacyLobeKindV1`; these aliases never create additional regions.
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const SensoryGrounding: Self = Self::PerceptualIntegration;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const MetabolicDrive: Self = Self::InteroceptiveMotivational;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const AuditorySpeech: Self = Self::SocialCommunication;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const GlyphVision: Self = Self::SocialCommunication;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const LexiconConcept: Self = Self::MultimodalAssociation;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const CoreAssociation: Self = Self::TemporalPredictive;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const EpisodicMemory: Self = Self::MemoryInterface;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const WorkingMemory: Self = Self::WorkingContextExecutive;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const MotorArbitration: Self = Self::ActionPlanning;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const HomeostaticRegulation: Self = Self::FlexibleReserve;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const LanguageExpansion: Self = Self::SocialCommunication;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const MathQuantity: Self = Self::MultimodalAssociation;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const NarrativeHistory: Self = Self::TemporalPredictive;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const SocialReasoning: Self = Self::SocialCommunication;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const SelfCriticUncertainty: Self = Self::WorkingContextExecutive;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const PlanningDream: Self = Self::TemporalPredictive;
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "use the v2 founder homologue")]
    pub const SpeechWritingMotor: Self = Self::ActionPlanning;

    pub const fn stable_id(self) -> LobeIndex {
        LobeIndex(self.raw())
    }

    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::PerceptualIntegration),
            2 => Ok(Self::InteroceptiveMotivational),
            3 => Ok(Self::MultimodalAssociation),
            4 => Ok(Self::TemporalPredictive),
            5 => Ok(Self::WorkingContextExecutive),
            6 => Ok(Self::MemoryInterface),
            7 => Ok(Self::ActionPlanning),
            8 => Ok(Self::SocialCommunication),
            9 => Ok(Self::FlexibleReserve),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }

    pub const fn purpose(self) -> &'static str {
        match self {
            LobeKind::PerceptualIntegration => "typed sensory and spatial integration",
            LobeKind::InteroceptiveMotivational => "interoceptive and motivational integration",
            LobeKind::MultimodalAssociation => "cross-modal association",
            LobeKind::TemporalPredictive => "temporal context and prediction",
            LobeKind::WorkingContextExecutive => "working context and executive control",
            LobeKind::MemoryInterface => "memory indexing and retrieval interface",
            LobeKind::ActionPlanning => "action planning and motor interface",
            LobeKind::SocialCommunication => "social and communication interface",
            LobeKind::FlexibleReserve => "developmentally recruitable flexible capacity",
        }
    }

    pub const fn default_update_cadence(self) -> UpdateCadence {
        match self {
            LobeKind::PerceptualIntegration | LobeKind::ActionPlanning => UpdateCadence::Hot60Hz,
            LobeKind::InteroceptiveMotivational => UpdateCadence::Hot10To30Hz,
            LobeKind::MultimodalAssociation
            | LobeKind::TemporalPredictive
            | LobeKind::WorkingContextExecutive
            | LobeKind::SocialCommunication => UpdateCadence::Hot15To60Hz,
            LobeKind::MemoryInterface | LobeKind::FlexibleReserve => UpdateCadence::Hot5To15Hz,
        }
    }

    pub const fn default_plasticity_policy(self) -> PlasticityPolicy {
        match self {
            LobeKind::PerceptualIntegration
            | LobeKind::InteroceptiveMotivational
            | LobeKind::ActionPlanning
            | LobeKind::SocialCommunication => PlasticityPolicy::Modulated,
            LobeKind::MultimodalAssociation
            | LobeKind::TemporalPredictive
            | LobeKind::WorkingContextExecutive => PlasticityPolicy::FastOjaHebbian,
            LobeKind::MemoryInterface | LobeKind::FlexibleReserve => {
                PlasticityPolicy::DecimatedOjaHebbian
            }
        }
    }

    pub const fn default_activation_policy(self) -> ActivationPolicy {
        match self {
            LobeKind::PerceptualIntegration
            | LobeKind::InteroceptiveMotivational
            | LobeKind::SocialCommunication => ActivationPolicy::InputCoupled,
            LobeKind::MultimodalAssociation
            | LobeKind::TemporalPredictive
            | LobeKind::WorkingContextExecutive
            | LobeKind::MemoryInterface
            | LobeKind::FlexibleReserve => ActivationPolicy::Recurrent,
            LobeKind::ActionPlanning => ActivationPolicy::OutputCoupled,
        }
    }

    pub const fn default_essentiality(self) -> LobeEssentiality {
        match self {
            LobeKind::PerceptualIntegration
            | LobeKind::InteroceptiveMotivational
            | LobeKind::ActionPlanning => LobeEssentiality::Essential,
            _ => LobeEssentiality::NonEssential,
        }
    }

    pub const fn default_throttle_priority(self) -> LobeThrottlePriority {
        match self {
            LobeKind::PerceptualIntegration
            | LobeKind::InteroceptiveMotivational
            | LobeKind::ActionPlanning => LobeThrottlePriority::Critical,
            LobeKind::MultimodalAssociation
            | LobeKind::TemporalPredictive
            | LobeKind::WorkingContextExecutive => LobeThrottlePriority::High,
            LobeKind::SocialCommunication => LobeThrottlePriority::Medium,
            LobeKind::MemoryInterface => LobeThrottlePriority::Low,
            LobeKind::FlexibleReserve => LobeThrottlePriority::SleepOnly,
        }
    }
}

/// The retired v1 region IDs. These exist only at explicit migration boundaries.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyLobeKindV1 {
    SensoryGrounding = 1,
    MetabolicDrive = 2,
    AuditorySpeech = 3,
    GlyphVision = 4,
    LexiconConcept = 5,
    CoreAssociation = 6,
    EpisodicMemory = 7,
    WorkingMemory = 8,
    MotorArbitration = 9,
    HomeostaticRegulation = 10,
    LanguageExpansion = 11,
    MathQuantity = 12,
    NarrativeHistory = 13,
    SocialReasoning = 14,
    SelfCriticUncertainty = 15,
    PlanningDream = 16,
    SpeechWritingMotor = 17,
}

impl LegacyLobeKindV1 {
    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::SensoryGrounding),
            2 => Ok(Self::MetabolicDrive),
            3 => Ok(Self::AuditorySpeech),
            4 => Ok(Self::GlyphVision),
            5 => Ok(Self::LexiconConcept),
            6 => Ok(Self::CoreAssociation),
            7 => Ok(Self::EpisodicMemory),
            8 => Ok(Self::WorkingMemory),
            9 => Ok(Self::MotorArbitration),
            10 => Ok(Self::HomeostaticRegulation),
            11 => Ok(Self::LanguageExpansion),
            12 => Ok(Self::MathQuantity),
            13 => Ok(Self::NarrativeHistory),
            14 => Ok(Self::SocialReasoning),
            15 => Ok(Self::SelfCriticUncertainty),
            16 => Ok(Self::PlanningDream),
            17 => Ok(Self::SpeechWritingMotor),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }

    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn migrate_to_founder(self) -> LobeKind {
        match self {
            Self::SensoryGrounding => LobeKind::PerceptualIntegration,
            Self::MetabolicDrive => LobeKind::InteroceptiveMotivational,
            Self::AuditorySpeech | Self::GlyphVision => LobeKind::SocialCommunication,
            Self::LexiconConcept | Self::MathQuantity => LobeKind::MultimodalAssociation,
            Self::CoreAssociation | Self::NarrativeHistory | Self::PlanningDream => {
                LobeKind::TemporalPredictive
            }
            Self::EpisodicMemory => LobeKind::MemoryInterface,
            Self::WorkingMemory | Self::SelfCriticUncertainty => LobeKind::WorkingContextExecutive,
            Self::MotorArbitration | Self::SpeechWritingMotor => LobeKind::ActionPlanning,
            Self::HomeostaticRegulation => LobeKind::FlexibleReserve,
            Self::LanguageExpansion | Self::SocialReasoning => LobeKind::SocialCommunication,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpdateCadence {
    Hot60Hz = 0,
    Hot15To60Hz = 1,
    Hot10To30Hz = 2,
    Hot5To15Hz = 3,
    Hot1To5Hz = 4,
    SleepOrOffline = 5,
    Disabled = 6,
}

impl UpdateCadence {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Hot60Hz),
            1 => Ok(Self::Hot15To60Hz),
            2 => Ok(Self::Hot10To30Hz),
            3 => Ok(Self::Hot5To15Hz),
            4 => Ok(Self::Hot1To5Hz),
            5 => Ok(Self::SleepOrOffline),
            6 => Ok(Self::Disabled),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
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
    InputCoupled,
    Recurrent,
    OutputCoupled,
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
        _neuron_count: u32,
        _disabled: LobeKind,
    ) -> Result<Self, ScaffoldContractError> {
        Err(ScaffoldContractError::PhenotypeCompile)
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

        if disabled.is_some() || mode != LayoutMode::Reference {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let layout = Self::reference_layout(neuron_count)?;
        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }

    fn reference_layout(neuron_count: u32) -> Result<Self, ScaffoldContractError> {
        let lengths = founder_floor_share_lengths(neuron_count)?;
        let regions = build_regions_from_lengths(neuron_count, &lengths);
        let layout = Self { regions };
        layout.validate_for_neuron_count(neuron_count)?;
        Ok(layout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Reference,
}

fn build_regions_from_lengths(_neuron_count: u32, lengths: &[u32; 9]) -> Vec<LobeRegion> {
    let mut regions = Vec::with_capacity(LobeKind::ALL.len());
    let mut start = 0;
    for (kind, len) in LobeKind::CORE.into_iter().zip(lengths.iter().copied()) {
        regions.push(LobeRegion::enabled(kind, start, len));
        start += len;
    }
    regions
}

fn founder_floor_share_lengths(neuron_count: u32) -> Result<[u32; 9], ScaffoldContractError> {
    if neuron_count < 512 {
        return Err(ScaffoldContractError::BrainClassTooSmall);
    }
    if !neuron_count.is_multiple_of(16) {
        return Err(ScaffoldContractError::LobeAlignment);
    }

    const FLOORS: [u32; 9] = [64, 32, 64, 48, 48, 32, 48, 32, 32];
    const SHARES: [u32; 9] = [16, 6, 23, 13, 10, 7, 10, 8, 7];
    let floor_total = FLOORS.iter().sum::<u32>();
    let remainder_blocks = (neuron_count - floor_total) / 16;
    let mut lengths = FLOORS;
    let mut assigned_blocks = 0_u32;
    let mut remainders = [(0_u32, 0_usize); 9];
    for index in 0..9 {
        let weighted = remainder_blocks * SHARES[index];
        let blocks = weighted / 100;
        lengths[index] += blocks * 16;
        assigned_blocks += blocks;
        remainders[index] = (weighted % 100, index);
    }
    remainders.sort_by(|left, right| right.cmp(left));
    for (_, index) in remainders
        .into_iter()
        .take((remainder_blocks - assigned_blocks) as usize)
    {
        lengths[index] += 16;
    }
    debug_assert_eq!(lengths.iter().sum::<u32>(), neuron_count);
    Ok(lengths)
}
