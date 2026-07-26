//! Stable, bounded language codes and speech-act protocol contracts.

use std::collections::BTreeSet;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::blake3_digest::{domain_hasher, Blake3Write};
use crate::{
    ActionKind, Blake3Digest, Confidence, ExperiencePatch, OrganismId, ScaffoldContractError,
    Validate, Vec3f,
};

const CODEBOOK_DOMAIN: &[u8] = b"alife.language-codebook.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LanguageCodebookId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct LanguageTokenId(u16);

impl LanguageTokenId {
    pub fn new(raw: u16) -> Result<Self, ScaffoldContractError> {
        (raw < LanguageCodebookV1::CODE_COUNT)
            .then_some(Self(raw))
            .ok_or(ScaffoldContractError::PhenotypeCompile)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct UtteranceId(u64);

impl UtteranceId {
    pub fn new(raw: u64) -> Result<Self, ScaffoldContractError> {
        (raw != 0)
            .then_some(Self(raw))
            .ok_or(ScaffoldContractError::InvalidId)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for UtteranceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UtteranceSourceKind {
    Player = 0,
    Creature = 1,
    Teacher = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerUtterance {
    pub utterance_id: UtteranceId,
    pub source_kind: UtteranceSourceKind,
    pub addressee: Option<OrganismId>,
    pub source_position: Vec3f,
    pub tokens: Vec<LanguageTokenId>,
}

impl PlayerUtterance {
    pub fn try_new(
        utterance_id: UtteranceId,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        tokens: Vec<LanguageTokenId>,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            utterance_id,
            source_kind: UtteranceSourceKind::Player,
            addressee,
            source_position,
            tokens,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for PlayerUtterance {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.source_kind != UtteranceSourceKind::Player
            || self.tokens.is_empty()
            || self.tokens.len() > usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS)
            || self.tokens.iter().any(|token| token.raw() == 0)
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        if let Some(addressee) = self.addressee {
            addressee.validate()?;
        }
        self.source_position.validate()?;
        Ok(())
    }
}

pub const SPEECH_TRANSLATION_MAX_SURFACE_CHARS: usize = 512;
pub const SPEECH_TRANSLATION_MAX_BINDINGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceTokenBinding {
    pub surface: String,
    pub token: LanguageTokenId,
}

impl SurfaceTokenBinding {
    pub fn try_new(
        surface: impl Into<String>,
        token: LanguageTokenId,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            surface: surface.into().trim().to_lowercase(),
            token,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for SurfaceTokenBinding {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.surface.is_empty()
            || self.surface.chars().count() > 64
            || self.surface.chars().any(char::is_whitespace)
            || self.token.raw() == 0
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechTranslationInput {
    PlayerText { text: String },
    CreatureTokens { tokens: Vec<LanguageTokenId> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechTranslationRequest {
    pub utterance_id: UtteranceId,
    pub addressee: Option<OrganismId>,
    pub input: SpeechTranslationInput,
    pub known_bindings: Vec<SurfaceTokenBinding>,
}

impl SpeechTranslationRequest {
    pub fn try_new(
        utterance_id: UtteranceId,
        addressee: Option<OrganismId>,
        input: SpeechTranslationInput,
        known_bindings: Vec<SurfaceTokenBinding>,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            utterance_id,
            addressee,
            input,
            known_bindings,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn raw_tokens(&self) -> Vec<LanguageTokenId> {
        match &self.input {
            SpeechTranslationInput::PlayerText { .. } => Vec::new(),
            SpeechTranslationInput::CreatureTokens { tokens } => tokens.clone(),
        }
    }
}

impl Validate for SpeechTranslationRequest {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if let Some(addressee) = self.addressee {
            addressee.validate()?;
        }
        if self.known_bindings.len() > SPEECH_TRANSLATION_MAX_BINDINGS {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        for binding in &self.known_bindings {
            binding.validate_contract()?;
        }
        let unique_surfaces = self
            .known_bindings
            .iter()
            .map(|binding| binding.surface.as_str())
            .collect::<BTreeSet<_>>();
        if unique_surfaces.len() != self.known_bindings.len() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        match &self.input {
            SpeechTranslationInput::PlayerText { text } => {
                let length = text.chars().count();
                if length == 0 || length > SPEECH_TRANSLATION_MAX_SURFACE_CHARS {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
            }
            SpeechTranslationInput::CreatureTokens { tokens } => {
                if tokens.is_empty()
                    || tokens.len() > usize::from(LanguageCodebookV1::MAX_GENERATED_TOKENS)
                    || tokens.iter().any(|token| token.raw() == 0)
                {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NovelLanguageToken {
    pub token: LanguageTokenId,
    pub surface: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechTranslationReceipt {
    pub utterance_id: UtteranceId,
    pub addressee: Option<OrganismId>,
    pub literal_tokens: Vec<LanguageTokenId>,
    pub novel_tokens: Vec<NovelLanguageToken>,
    pub literal_text: String,
    pub rendered_text: String,
    pub confidence: Confidence,
    pub model_identity: String,
    pub assisted: bool,
    pub uncertain: bool,
}

impl Validate for SpeechTranslationReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if let Some(addressee) = self.addressee {
            addressee.validate()?;
        }
        if self.literal_tokens.is_empty()
            || self.literal_tokens.len() > usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS)
            || self.literal_tokens.iter().any(|token| token.raw() == 0)
            || self.novel_tokens.len() > self.literal_tokens.len()
            || self.novel_tokens.iter().any(|novel| {
                novel.surface.trim().is_empty()
                    || novel.surface.chars().count() > 64
                    || !self.literal_tokens.contains(&novel.token)
            })
            || self.literal_text.trim().is_empty()
            || self.rendered_text.trim().is_empty()
            || self.model_identity.trim().is_empty()
            || self.model_identity.chars().count() > 128
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechMotorPayload {
    pub speech_act: SpeechActKind,
    pub tokens: Vec<LanguageTokenId>,
    pub confidence: Confidence,
}

pub const LANGUAGE_GROUNDING_LEDGER_CAPACITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageGroundingEntry {
    pub token: LanguageTokenId,
    pub action: ActionKind,
    pub exposures: u32,
    pub successful_outcomes: u32,
}

/// Bounded, non-authoritative associations observed only from sealed life
/// experience. The ledger never supplies action scores or neural activations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageGroundingLedger {
    last_sequence_id: u64,
    entries: Vec<LanguageGroundingEntry>,
}

impl LanguageGroundingLedger {
    pub fn entries(&self) -> &[LanguageGroundingEntry] {
        &self.entries
    }

    pub const fn last_sequence_id(&self) -> u64 {
        self.last_sequence_id
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.entries.len() > LANGUAGE_GROUNDING_LEDGER_CAPACITY
            || (!self.entries.is_empty() && self.last_sequence_id == 0)
            || self.entries.iter().any(|entry| {
                entry.token.raw() == 0
                    || entry.exposures == 0
                    || entry.successful_outcomes > entry.exposures
            })
            || self.entries.windows(2).any(|pair| {
                (pair[0].token.raw(), pair[0].action.raw())
                    >= (pair[1].token.raw(), pair[1].action.raw())
            })
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }

    pub fn observe_sealed(&mut self, patch: &ExperiencePatch) -> Result<(), ScaffoldContractError> {
        patch.validate_contract()?;
        let sequence = patch.header().sequence_id.raw();
        if sequence <= self.last_sequence_id {
            return Err(ScaffoldContractError::LearningReplayRejected);
        }
        self.last_sequence_id = sequence;
        let action = patch.decision().selected_action.kind;
        let succeeded = patch.outcome().success;
        let mut tokens = patch
            .pre_action()
            .perception()
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .filter_map(|heard| u16::try_from(heard.token_id).ok())
            .filter_map(|raw| LanguageTokenId::new(raw).ok())
            .filter(|token| token.raw() != 0)
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        tokens.dedup();
        for token in tokens {
            if let Some(entry) = self
                .entries
                .iter_mut()
                .find(|entry| entry.token == token && entry.action == action)
            {
                entry.exposures = entry.exposures.saturating_add(1);
                entry.successful_outcomes = entry
                    .successful_outcomes
                    .saturating_add(u32::from(succeeded));
                continue;
            }
            if self.entries.len() == LANGUAGE_GROUNDING_LEDGER_CAPACITY {
                let eviction = self
                    .entries
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, entry)| {
                        (entry.exposures, entry.token.raw(), entry.action.raw())
                    })
                    .map(|(index, _)| index)
                    .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
                self.entries.remove(eviction);
            }
            self.entries.push(LanguageGroundingEntry {
                token,
                action,
                exposures: 1,
                successful_outcomes: u32::from(succeeded),
            });
        }
        self.entries
            .sort_by_key(|entry| (entry.token.raw(), entry.action.raw()));
        self.validate_contract()?;
        Ok(())
    }
}

impl SpeechMotorPayload {
    pub fn try_new(
        speech_act: SpeechActKind,
        tokens: Vec<LanguageTokenId>,
        confidence: Confidence,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            speech_act,
            tokens,
            confidence,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for SpeechMotorPayload {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.tokens.is_empty()
            || self.tokens.len() > usize::from(LanguageCodebookV1::MAX_GENERATED_TOKENS)
            || self.tokens.iter().any(|token| token.raw() == 0)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

impl<'de> Deserialize<'de> for LanguageTokenId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = u16::deserialize(deserializer)?;
        Self::new(raw).map_err(D::Error::custom)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageTokenClass {
    SilenceUnknown = 0,
    VerbAction = 1,
    EcologicalNoun = 2,
    DriveInternalState = 3,
    ModifierSpatialRelation = 4,
    GrammarQuerySocialOperator = 5,
    LearnedAliasDialect = 6,
    NameSocialBinding = 7,
    ReservedExperimental = 8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpeechActKind {
    Declare = 0,
    Request = 1,
    Respond = 2,
    QueryWhat = 3,
    QueryWhy = 4,
    ExpressState = 5,
    Acknowledge = 6,
    Refuse = 7,
}

impl SpeechActKind {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Declare),
            1 => Ok(Self::Request),
            2 => Ok(Self::Respond),
            3 => Ok(Self::QueryWhat),
            4 => Ok(Self::QueryWhy),
            5 => Ok(Self::ExpressState),
            6 => Ok(Self::Acknowledge),
            7 => Ok(Self::Refuse),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

/// Frozen recurrent speech-head lane roles. The head emits one 8-bit token per
/// recurrent decode step and stops after at most six steps.
pub struct SpeechDecoderLayoutV1;

impl SpeechDecoderLayoutV1 {
    pub const SCHEMA_VERSION: u16 = 1;
    pub const INPUT_WIDTH: u16 = 32;
    pub const OUTPUT_WIDTH: u16 = 32;
    pub const MOTOR_SOURCE_OFFSET: u32 = 128;
    pub const MOTOR_TARGET_OFFSET: u32 = 160;
    pub const SPEECH_ACT_START: u16 = 0;
    pub const SPEECH_ACT_COUNT: u16 = 8;
    pub const TOKEN_BIT_START: u16 = 8;
    pub const TOKEN_BIT_COUNT: u16 = 8;
    pub const EMIT_OUTPUT: u16 = 16;
    pub const STOP_OUTPUT: u16 = 17;
    pub const RECURRENT_CONTROL_START: u16 = 18;
    pub const RECURRENT_CONTROL_COUNT: u16 = 14;

    pub const fn speech_act_output(act: SpeechActKind) -> u16 {
        Self::SPEECH_ACT_START + act.raw() as u16
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageCodebookV1 {
    schema_version: u16,
    id: LanguageCodebookId,
    code_count: u16,
    max_heard_tokens: u8,
    max_generated_tokens: u8,
    canonical_digest: Blake3Digest,
}

impl LanguageCodebookV1 {
    pub const CODE_COUNT: u16 = 256;
    pub const MAX_HEARD_TOKENS: u8 = 16;
    pub const MAX_GENERATED_TOKENS: u8 = 6;

    pub fn canonical() -> Self {
        let mut value = Self {
            schema_version: 1,
            id: LanguageCodebookId(0x4C43_5631),
            code_count: Self::CODE_COUNT,
            max_heard_tokens: Self::MAX_HEARD_TOKENS,
            max_generated_tokens: Self::MAX_GENERATED_TOKENS,
            canonical_digest: Blake3Digest::default(),
        };
        value.canonical_digest = value.recompute_digest();
        value
    }

    pub const fn id(&self) -> LanguageCodebookId {
        self.id
    }
    pub const fn code_count(&self) -> u16 {
        self.code_count
    }
    pub const fn max_heard_tokens(&self) -> u8 {
        self.max_heard_tokens
    }
    pub const fn max_generated_tokens(&self) -> u8 {
        self.max_generated_tokens
    }
    pub const fn canonical_digest(&self) -> Blake3Digest {
        self.canonical_digest
    }

    pub const fn classify(&self, token: LanguageTokenId) -> LanguageTokenClass {
        match token.raw() {
            0 => LanguageTokenClass::SilenceUnknown,
            1..=24 => LanguageTokenClass::VerbAction,
            25..=88 => LanguageTokenClass::EcologicalNoun,
            89..=104 => LanguageTokenClass::DriveInternalState,
            105..=120 => LanguageTokenClass::ModifierSpatialRelation,
            121..=128 => LanguageTokenClass::GrammarQuerySocialOperator,
            129..=192 => LanguageTokenClass::LearnedAliasDialect,
            193..=224 => LanguageTokenClass::NameSocialBinding,
            225..=255 => LanguageTokenClass::ReservedExperimental,
            _ => unreachable!(),
        }
    }

    /// Returns the stable content-neutral vocal symbol for a logical token.
    /// Localized words and learned meanings are separate runtime bindings.
    pub fn pronounceable_symbol(&self, token: LanguageTokenId) -> String {
        const ONSETS: [&str; 16] = [
            "b", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "r", "s", "t", "v", "z",
        ];
        const RHYMES: [&str; 16] = [
            "ala", "ami", "ano", "aru", "ela", "emi", "eno", "eru", "ila", "imi", "ino", "iru",
            "ola", "omi", "ono", "oru",
        ];
        if token.raw() == 0 {
            return "sil".to_owned();
        }
        let raw = usize::from(token.raw());
        format!("{}{}", ONSETS[raw / 16], RHYMES[raw % 16])
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != 1
            || self.id != LanguageCodebookId(0x4C43_5631)
            || self.code_count != Self::CODE_COUNT
            || self.max_heard_tokens != Self::MAX_HEARD_TOKENS
            || self.max_generated_tokens != Self::MAX_GENERATED_TOKENS
            || self.canonical_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Blake3Digest {
        let mut h = domain_hasher(CODEBOOK_DOMAIN);
        h.write_u16(self.schema_version);
        h.write_u32(self.id.0);
        h.write_u16(self.code_count);
        h.write_u8(self.max_heard_tokens);
        h.write_u8(self.max_generated_tokens);
        for value in [
            SpeechDecoderLayoutV1::SCHEMA_VERSION,
            SpeechDecoderLayoutV1::INPUT_WIDTH,
            SpeechDecoderLayoutV1::OUTPUT_WIDTH,
            SpeechDecoderLayoutV1::SPEECH_ACT_START,
            SpeechDecoderLayoutV1::SPEECH_ACT_COUNT,
            SpeechDecoderLayoutV1::TOKEN_BIT_START,
            SpeechDecoderLayoutV1::TOKEN_BIT_COUNT,
            SpeechDecoderLayoutV1::EMIT_OUTPUT,
            SpeechDecoderLayoutV1::STOP_OUTPUT,
            SpeechDecoderLayoutV1::RECURRENT_CONTROL_START,
            SpeechDecoderLayoutV1::RECURRENT_CONTROL_COUNT,
        ] {
            h.write_u16(value);
        }
        h.write_u32(SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET);
        h.write_u32(SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET);
        for raw in 0..Self::CODE_COUNT {
            let token = LanguageTokenId(raw);
            h.write_u16(raw);
            h.write_u8(self.classify(token) as u8);
            let symbol = self.pronounceable_symbol(token);
            h.write_len(symbol.len());
            h.update(symbol.as_bytes());
        }
        Blake3Digest::from_hasher(h)
    }
}

impl<'de> Deserialize<'de> for LanguageCodebookV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            id: LanguageCodebookId,
            code_count: u16,
            max_heard_tokens: u8,
            max_generated_tokens: u8,
            canonical_digest: Blake3Digest,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            id: w.id,
            code_count: w.code_count,
            max_heard_tokens: w.max_heard_tokens,
            max_generated_tokens: w.max_generated_tokens,
            canonical_digest: w.canonical_digest,
        };
        value.validate_contract().map_err(D::Error::custom)?;
        Ok(value)
    }
}
