//! v0 scaffold: engine-neutral sensory ABI, context streams, and perception refs.

use core::ops::{BitOr, BitOrAssign};

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_optional_target, ConceptCellId, Confidence, GaussianClusterId,
    NormalizedScalar, OrganismId, ScaffoldContractError, SchemaKind, SchemaVersions, SignedValence,
    Tick, Validate, Vec3f, WorldEntityId,
};

pub const SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT: usize = 16;
pub const SENSORY_AUDITORY_CHANNEL_COUNT: usize = 8;
pub const SENSORY_SMELL_CHANNEL_COUNT: usize = 8;
pub const SENSORY_TACTILE_CHANNEL_COUNT: usize = 8;
pub const SENSORY_PAIN_NOVELTY_CHANNEL_COUNT: usize = 2;
pub const SENSORY_ABI_CHANNEL_COUNT: usize = SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT
    + SENSORY_AUDITORY_CHANNEL_COUNT
    + SENSORY_SMELL_CHANNEL_COUNT
    + SENSORY_TACTILE_CHANNEL_COUNT
    + SENSORY_PAIN_NOVELTY_CHANNEL_COUNT;

pub const MAX_HEARD_TOKENS: usize = 16;
pub const MAX_SOCIAL_AGENTS: usize = 8;
pub const MAX_OPTIONAL_ENVIRONMENT_STREAMS: usize = 8;

const MIN_ATMOSPHERIC_TEMPERATURE_CELSIUS: f32 = -100.0;
const MAX_ATMOSPHERIC_TEMPERATURE_CELSIUS: f32 = 150.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SensoryAbiVersion(pub u16);

impl SensoryAbiVersion {
    pub const CURRENT: Self = Self(SchemaVersions::CURRENT.sensory_abi.0);

    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl Default for SensoryAbiVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelGroupKind {
    VisualAffordance,
    AuditoryAcoustic,
    SmellChemistry,
    TactileContact,
    PainNovelty,
    NearbyAffordanceBitfield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelBounds {
    NormalizedUnit,
    Bitfield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelExtensionPolicy {
    FrozenForVersion,
    AppendOnlyWithVersionBump,
    OptionalSideBuffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelGroupSpec {
    pub kind: ChannelGroupKind,
    pub channel_count: usize,
    pub semantics: &'static str,
    pub bounds: ChannelBounds,
    pub extension_policy: ChannelExtensionPolicy,
}

pub const SENSORY_ABI_CHANNEL_GROUPS: [ChannelGroupSpec; 6] = [
    ChannelGroupSpec {
        kind: ChannelGroupKind::VisualAffordance,
        channel_count: SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
        semantics: "egocentric visual affordance salience such as food, hazard, mate, shelter, tool, glyph, and teacher object cues",
        bounds: ChannelBounds::NormalizedUnit,
        extension_policy: ChannelExtensionPolicy::AppendOnlyWithVersionBump,
    },
    ChannelGroupSpec {
        kind: ChannelGroupKind::AuditoryAcoustic,
        channel_count: SENSORY_AUDITORY_CHANNEL_COUNT,
        semantics: "auditory loudness, token/phoneme confidence, speaker salience, prosody, and acoustic noise",
        bounds: ChannelBounds::NormalizedUnit,
        extension_policy: ChannelExtensionPolicy::AppendOnlyWithVersionBump,
    },
    ChannelGroupSpec {
        kind: ChannelGroupKind::SmellChemistry,
        channel_count: SENSORY_SMELL_CHANNEL_COUNT,
        semantics: "smell and local chemistry cues including food, pheromone, danger, decay, and resource gradients",
        bounds: ChannelBounds::NormalizedUnit,
        extension_policy: ChannelExtensionPolicy::AppendOnlyWithVersionBump,
    },
    ChannelGroupSpec {
        kind: ChannelGroupKind::TactileContact,
        channel_count: SENSORY_TACTILE_CHANNEL_COUNT,
        semantics: "contact pressure, collision, grip, ground, fluid, temperature contact, and touch cues",
        bounds: ChannelBounds::NormalizedUnit,
        extension_policy: ChannelExtensionPolicy::AppendOnlyWithVersionBump,
    },
    ChannelGroupSpec {
        kind: ChannelGroupKind::PainNovelty,
        channel_count: SENSORY_PAIN_NOVELTY_CHANNEL_COUNT,
        semantics: "bounded pain and novelty signals that can modulate attention without bypassing action arbitration",
        bounds: ChannelBounds::NormalizedUnit,
        extension_policy: ChannelExtensionPolicy::FrozenForVersion,
    },
    ChannelGroupSpec {
        kind: ChannelGroupKind::NearbyAffordanceBitfield,
        channel_count: 0,
        semantics: "nearby affordance presence bits carried as a stable bitfield beside normalized numeric channels",
        bounds: ChannelBounds::Bitfield,
        extension_policy: ChannelExtensionPolicy::OptionalSideBuffer,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensoryAbiDescriptor {
    pub version: SensoryAbiVersion,
    pub channel_groups: &'static [ChannelGroupSpec],
}

impl SensoryAbiDescriptor {
    pub const V1: Self = Self {
        version: SensoryAbiVersion::CURRENT,
        channel_groups: &SENSORY_ABI_CHANNEL_GROUPS,
    };

    pub fn total_channel_count(&self) -> usize {
        self.channel_groups
            .iter()
            .map(|group| group.channel_count)
            .sum()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AffordanceBits(pub u32);

impl AffordanceBits {
    pub const NONE: Self = Self(0);
    pub const FOOD: Self = Self(1 << 0);
    pub const WATER: Self = Self(1 << 1);
    pub const HAZARD: Self = Self(1 << 2);
    pub const MATE: Self = Self(1 << 3);
    pub const SOCIAL_AGENT: Self = Self(1 << 4);
    pub const SHELTER: Self = Self(1 << 5);
    pub const TOOL: Self = Self(1 << 6);
    pub const GLYPH_OR_WRITING: Self = Self(1 << 7);
    pub const TEACHER_OBJECT: Self = Self(1 << 8);
    pub const RESOURCE: Self = Self(1 << 9);

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Default for AffordanceBits {
    fn default() -> Self {
        Self::NONE
    }
}

impl BitOr for AffordanceBits {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for AffordanceBits {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextFeatureFlags(pub u32);

impl ContextFeatureFlags {
    pub const NONE: Self = Self(0);
    pub const GAUSSIAN_CLUSTERS: Self = Self(1 << 0);
    pub const EGOCENTRIC_BIN_HASH: Self = Self(1 << 1);
    pub const SEMANTIC_CODES: Self = Self(1 << 2);
    pub const INTERNAL_SLM_MODULATION: Self = Self(1 << 3);
    pub const TEACHER_PERCEPTION_MARKER: Self = Self(1 << 4);
    pub const SCHOOL_PERCEPTION_MARKER: Self = Self(1 << 5);

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Default for ContextFeatureFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl BitOr for ContextFeatureFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ContextFeatureFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensoryChannels {
    pub visual_affordance: [f32; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT],
    pub auditory_acoustic: [f32; SENSORY_AUDITORY_CHANNEL_COUNT],
    pub smell_chemistry: [f32; SENSORY_SMELL_CHANNEL_COUNT],
    pub tactile_contact: [f32; SENSORY_TACTILE_CHANNEL_COUNT],
    pub pain_signal: NormalizedScalar,
    pub novelty_signal: NormalizedScalar,
    pub nearby_affordances: AffordanceBits,
}

impl SensoryChannels {
    pub const ZERO: Self = Self {
        visual_affordance: [0.0; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT],
        auditory_acoustic: [0.0; SENSORY_AUDITORY_CHANNEL_COUNT],
        smell_chemistry: [0.0; SENSORY_SMELL_CHANNEL_COUNT],
        tactile_contact: [0.0; SENSORY_TACTILE_CHANNEL_COUNT],
        pain_signal: NormalizedScalar(0.0),
        novelty_signal: NormalizedScalar(0.0),
        nearby_affordances: AffordanceBits::NONE,
    };

    pub fn try_from_groups(
        visual_affordance: [f32; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT],
        auditory_acoustic: [f32; SENSORY_AUDITORY_CHANNEL_COUNT],
        smell_chemistry: [f32; SENSORY_SMELL_CHANNEL_COUNT],
        tactile_contact: [f32; SENSORY_TACTILE_CHANNEL_COUNT],
        pain_signal: NormalizedScalar,
        novelty_signal: NormalizedScalar,
        nearby_affordances: AffordanceBits,
    ) -> Result<Self, ScaffoldContractError> {
        let channels = Self {
            visual_affordance,
            auditory_acoustic,
            smell_chemistry,
            tactile_contact,
            pain_signal,
            novelty_signal,
            nearby_affordances,
        };
        channels.validate_contract()?;
        Ok(channels)
    }

    pub fn as_flat_array(&self) -> [f32; SENSORY_ABI_CHANNEL_COUNT] {
        let mut out = [0.0; SENSORY_ABI_CHANNEL_COUNT];
        let mut offset = 0;

        out[offset..offset + SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT]
            .copy_from_slice(&self.visual_affordance);
        offset += SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT;

        out[offset..offset + SENSORY_AUDITORY_CHANNEL_COUNT]
            .copy_from_slice(&self.auditory_acoustic);
        offset += SENSORY_AUDITORY_CHANNEL_COUNT;

        out[offset..offset + SENSORY_SMELL_CHANNEL_COUNT].copy_from_slice(&self.smell_chemistry);
        offset += SENSORY_SMELL_CHANNEL_COUNT;

        out[offset..offset + SENSORY_TACTILE_CHANNEL_COUNT].copy_from_slice(&self.tactile_contact);
        offset += SENSORY_TACTILE_CHANNEL_COUNT;

        out[offset] = self.pain_signal.raw();
        out[offset + 1] = self.novelty_signal.raw();

        out
    }
}

impl Default for SensoryChannels {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Validate for SensoryChannels {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized_slice(&self.visual_affordance)?;
        validate_normalized_slice(&self.auditory_acoustic)?;
        validate_normalized_slice(&self.smell_chemistry)?;
        validate_normalized_slice(&self.tactile_contact)?;
        validate_normalized(self.pain_signal.raw())?;
        validate_normalized(self.novelty_signal.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentStreamEntry {
    pub stream_id: u16,
    pub value: NormalizedScalar,
    pub confidence: Confidence,
}

impl Validate for EnvironmentStreamEntry {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.stream_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        validate_normalized(self.value.raw())?;
        validate_confidence(self.confidence)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocialProximityEntry {
    pub agent_id: OrganismId,
    pub proximity: NormalizedScalar,
    pub confidence: Confidence,
}

impl Validate for SocialProximityEntry {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.agent_id.validate()?;
        validate_normalized(self.proximity.raw())?;
        validate_confidence(self.confidence)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContextStreams {
    pub abi_version: SensoryAbiVersion,
    pub atmospheric_temperature_celsius: f32,
    pub ambient_light: NormalizedScalar,
    pub energy_intake_trend: SignedValence,
    pub blood_sugar_trend: SignedValence,
    pub vocal_tokens: [Option<HeardToken>; MAX_HEARD_TOKENS],
    pub social_proximity: [Option<SocialProximityEntry>; MAX_SOCIAL_AGENTS],
    pub optional_environment: [Option<EnvironmentStreamEntry>; MAX_OPTIONAL_ENVIRONMENT_STREAMS],
}

impl Default for ContextStreams {
    fn default() -> Self {
        Self {
            abi_version: SensoryAbiVersion::CURRENT,
            atmospheric_temperature_celsius: 20.0,
            ambient_light: NormalizedScalar(0.0),
            energy_intake_trend: SignedValence(0.0),
            blood_sugar_trend: SignedValence(0.0),
            vocal_tokens: [None; MAX_HEARD_TOKENS],
            social_proximity: [None; MAX_SOCIAL_AGENTS],
            optional_environment: [None; MAX_OPTIONAL_ENVIRONMENT_STREAMS],
        }
    }
}

impl Validate for ContextStreams {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::SensoryAbi, self.abi_version.raw())?;
        validate_temperature_celsius(self.atmospheric_temperature_celsius)?;
        validate_normalized(self.ambient_light.raw())?;
        validate_signed_unit(self.energy_intake_trend.raw())?;
        validate_signed_unit(self.blood_sugar_trend.raw())?;
        validate_option_array(&self.vocal_tokens)?;
        validate_option_array(&self.social_proximity)?;
        validate_option_array(&self.optional_environment)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeacherPerceptionChannel {
    Hearing,
    Vision,
    Writing,
    Gesture,
    Object,
}

impl TeacherPerceptionChannel {
    pub const ALL: [TeacherPerceptionChannel; 5] = [
        TeacherPerceptionChannel::Hearing,
        TeacherPerceptionChannel::Vision,
        TeacherPerceptionChannel::Writing,
        TeacherPerceptionChannel::Gesture,
        TeacherPerceptionChannel::Object,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeardToken {
    pub speaker_id: Option<OrganismId>,
    pub source_entity: Option<WorldEntityId>,
    pub token_id: u32,
    pub source_position: Vec3f,
    pub confidence: Confidence,
    pub teacher_channel: Option<TeacherPerceptionChannel>,
}

impl Validate for HeardToken {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_optional_organism_id(self.speaker_id)?;
        validate_optional_target(self.source_entity)?;
        if self.token_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.source_position.validate()?;
        validate_confidence(self.confidence)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VocalizedToken {
    pub token_id: u32,
    pub confidence: Confidence,
}

impl Validate for VocalizedToken {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.token_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        validate_confidence(self.confidence)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LanguageContextSnapshot {
    pub heard_tokens: [Option<HeardToken>; MAX_HEARD_TOKENS],
    pub vocalized_token: Option<VocalizedToken>,
    pub word_confidence: Confidence,
    pub teacher_channel_marker: Option<TeacherPerceptionChannel>,
}

impl Default for LanguageContextSnapshot {
    fn default() -> Self {
        Self {
            heard_tokens: [None; MAX_HEARD_TOKENS],
            vocalized_token: None,
            word_confidence: Confidence(0.0),
            teacher_channel_marker: None,
        }
    }
}

impl Validate for LanguageContextSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_option_array(&self.heard_tokens)?;
        if let Some(token) = self.vocalized_token {
            token.validate_contract()?;
        }
        validate_confidence(self.word_confidence)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocialAgentSnapshot {
    pub agent_id: OrganismId,
    pub body_entity: Option<WorldEntityId>,
    pub relative_position: Vec3f,
    pub gaze_direction: Vec3f,
    pub orientation_forward: Vec3f,
    pub affinity: SignedValence,
    pub proximity: NormalizedScalar,
}

impl Validate for SocialAgentSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.agent_id.validate()?;
        validate_optional_target(self.body_entity)?;
        self.relative_position.validate()?;
        self.gaze_direction.validate()?;
        self.orientation_forward.validate()?;
        validate_signed_unit(self.affinity.raw())?;
        validate_normalized(self.proximity.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocialContextSnapshot {
    pub nearest_agents: [Option<SocialAgentSnapshot>; MAX_SOCIAL_AGENTS],
}

impl Default for SocialContextSnapshot {
    fn default() -> Self {
        Self {
            nearest_agents: [None; MAX_SOCIAL_AGENTS],
        }
    }
}

impl Validate for SocialContextSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_option_array(&self.nearest_agents)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianSalienceEntry {
    pub cluster_id: GaussianClusterId,
    pub salience: NormalizedScalar,
    pub distance_meters: f32,
}

impl Validate for GaussianSalienceEntry {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.cluster_id.validate()?;
        validate_normalized(self.salience.raw())?;
        validate_nonnegative_finite(self.distance_meters)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianContextRef {
    pub egocentric_bin_hash: u64,
    pub feature_flags: ContextFeatureFlags,
    pub confidence: Confidence,
    pub clusters: Vec<GaussianSalienceEntry>,
}

impl Validate for GaussianContextRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_confidence(self.confidence)?;
        for cluster in &self.clusters {
            cluster.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressedSemanticCode {
    pub codebook_id: u16,
    pub code: u32,
    pub salience: NormalizedScalar,
}

impl Validate for CompressedSemanticCode {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.codebook_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        validate_normalized(self.salience.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticSalienceEntry {
    pub concept_id: ConceptCellId,
    pub salience: NormalizedScalar,
}

impl Validate for SemanticSalienceEntry {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.concept_id.validate()?;
        validate_normalized(self.salience.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticContextRef {
    pub feature_flags: ContextFeatureFlags,
    pub confidence: Confidence,
    pub compressed_codes: Vec<CompressedSemanticCode>,
    pub salience: Vec<SemanticSalienceEntry>,
}

impl Validate for SemanticContextRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_confidence(self.confidence)?;
        for code in &self.compressed_codes {
            code.validate_contract()?;
        }
        for entry in &self.salience {
            entry.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorySnapshot {
    pub abi_version: SensoryAbiVersion,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub observer_position: Vec3f,
    pub channels: SensoryChannels,
    pub context_streams: ContextStreams,
    pub social_context: SocialContextSnapshot,
    pub language_context: LanguageContextSnapshot,
    pub semantic_context: Option<SemanticContextRef>,
    pub gaussian_context: Option<GaussianContextRef>,
}

impl SensorySnapshot {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        observer_position: Vec3f,
        channels: SensoryChannels,
        context_streams: ContextStreams,
    ) -> Result<Self, ScaffoldContractError> {
        let snapshot = Self {
            abi_version: SensoryAbiVersion::CURRENT,
            organism_id,
            tick,
            observer_position,
            channels,
            context_streams,
            social_context: SocialContextSnapshot::default(),
            language_context: LanguageContextSnapshot::default(),
            semantic_context: None,
            gaussian_context: None,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }
}

impl Validate for SensorySnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::SensoryAbi, self.abi_version.raw())?;
        self.organism_id.validate()?;
        self.observer_position.validate()?;
        self.channels.validate_contract()?;
        self.context_streams.validate_contract()?;
        self.social_context.validate_contract()?;
        self.language_context.validate_contract()?;
        if let Some(context) = &self.semantic_context {
            context.validate_contract()?;
        }
        if let Some(context) = &self.gaussian_context {
            context.validate_contract()?;
        }
        Ok(())
    }
}

pub trait SensorySnapshotFromAdapter<T>: Sized {
    fn sensory_from_adapter(value: T) -> Result<Self, ScaffoldContractError>;
}

pub trait SensorySnapshotSource {
    fn sensory_snapshot(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<SensorySnapshot, ScaffoldContractError>;
}

fn validate_option_array<T: Validate, const N: usize>(
    values: &[Option<T>; N],
) -> Result<(), ScaffoldContractError> {
    for value in values.iter().flatten() {
        value.validate_contract()?;
    }
    Ok(())
}

fn validate_optional_organism_id(id: Option<OrganismId>) -> Result<(), ScaffoldContractError> {
    if let Some(id) = id {
        id.validate()?;
    }
    Ok(())
}

fn validate_normalized_slice(values: &[f32]) -> Result<(), ScaffoldContractError> {
    for value in values {
        validate_normalized(*value)?;
    }
    Ok(())
}

fn validate_normalized(value: f32) -> Result<(), ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_signed_unit(value: f32) -> Result<(), ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    if (-1.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_confidence(value: Confidence) -> Result<(), ScaffoldContractError> {
    validate_normalized(value.raw())
}

fn validate_temperature_celsius(value: f32) -> Result<(), ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    if (MIN_ATMOSPHERIC_TEMPERATURE_CELSIUS..=MAX_ATMOSPHERIC_TEMPERATURE_CELSIUS).contains(&value)
    {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_nonnegative_finite(value: f32) -> Result<(), ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    if value >= 0.0 {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}
