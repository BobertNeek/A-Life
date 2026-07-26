//! Spatial, bounded raw-token hearing owned by the world.

use std::collections::BTreeMap;

use alife_core::{
    Confidence, HeardToken, LanguageCodebookV1, LanguageTokenId, OrganismId, PlayerUtterance,
    ScaffoldContractError, SpeechMotorPayload, TeacherPerceptionChannel, Tick, UtteranceId,
    UtteranceSourceKind, Validate, Vec3f,
};

pub const DEFAULT_SPEECH_HEARING_RADIUS: f32 = 6.0;
pub const DEFAULT_UTTERANCE_LIFETIME_TICKS: u64 = 1;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudibleUtterance {
    pub utterance_id: UtteranceId,
    pub source_kind: UtteranceSourceKind,
    pub speaker_id: Option<OrganismId>,
    pub addressee: Option<OrganismId>,
    pub source_position: Vec3f,
    pub tokens: Vec<LanguageTokenId>,
    pub confidence: Confidence,
    pub teacher_channel: Option<TeacherPerceptionChannel>,
    pub emitted_tick: Tick,
    pub expires_after_tick: Tick,
}

impl AudibleUtterance {
    pub fn from_player(
        value: PlayerUtterance,
        emitted_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        value.validate_contract()?;
        Self::try_new(
            value.utterance_id,
            value.source_kind,
            None,
            value.addressee,
            value.source_position,
            value.tokens,
            Confidence::new(1.0)?,
            None,
            emitted_tick,
        )
    }

    pub fn from_creature(
        utterance_id: UtteranceId,
        speaker_id: OrganismId,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        payload: SpeechMotorPayload,
        emitted_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        payload.validate_contract()?;
        Self::try_new(
            utterance_id,
            UtteranceSourceKind::Creature,
            Some(speaker_id),
            addressee,
            source_position,
            payload.tokens,
            payload.confidence,
            None,
            emitted_tick,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_new(
        utterance_id: UtteranceId,
        source_kind: UtteranceSourceKind,
        speaker_id: Option<OrganismId>,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        tokens: Vec<LanguageTokenId>,
        confidence: Confidence,
        teacher_channel: Option<TeacherPerceptionChannel>,
        emitted_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            utterance_id,
            source_kind,
            speaker_id,
            addressee,
            source_position,
            tokens,
            confidence,
            teacher_channel,
            emitted_tick,
            expires_after_tick: Tick::new(
                emitted_tick
                    .raw()
                    .saturating_add(DEFAULT_UTTERANCE_LIFETIME_TICKS),
            ),
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for AudibleUtterance {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.tokens.is_empty()
            || self.tokens.len() > usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS)
            || self.tokens.iter().any(|token| token.raw() == 0)
            || self.expires_after_tick.raw() < self.emitted_tick.raw()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        if let Some(speaker_id) = self.speaker_id {
            speaker_id.validate()?;
        }
        if let Some(addressee) = self.addressee {
            addressee.validate()?;
        }
        if self.source_kind == UtteranceSourceKind::Creature && self.speaker_id.is_none() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        self.source_position.validate()?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpatialSpeechBus {
    utterances: BTreeMap<u64, AudibleUtterance>,
}

impl SpatialSpeechBus {
    pub fn restore(
        utterances: Vec<AudibleUtterance>,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let mut bus = Self::default();
        for utterance in utterances {
            if utterance.emitted_tick.raw() > tick.raw()
                || utterance.expires_after_tick.raw() < tick.raw()
            {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            bus.emit(utterance)?;
        }
        Ok(bus)
    }

    pub fn snapshot(&self) -> Vec<AudibleUtterance> {
        self.utterances.values().cloned().collect()
    }

    pub fn emit(&mut self, utterance: AudibleUtterance) -> Result<(), ScaffoldContractError> {
        utterance.validate_contract()?;
        if self
            .utterances
            .insert(utterance.utterance_id.raw(), utterance)
            .is_some()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn retire_expired(&mut self, tick: Tick) {
        self.utterances
            .retain(|_, utterance| utterance.expires_after_tick.raw() >= tick.raw());
    }

    pub fn heard_tokens(
        &self,
        listener: OrganismId,
        listener_position: Vec3f,
        tick: Tick,
    ) -> Result<Vec<HeardToken>, ScaffoldContractError> {
        let mut heard = Vec::new();
        for utterance in self.utterances.values() {
            if tick.raw() < utterance.emitted_tick.raw()
                || tick.raw() > utterance.expires_after_tick.raw()
                || utterance.speaker_id == Some(listener)
                || utterance.addressee.is_some_and(|value| value != listener)
            {
                continue;
            }
            let distance = distance(listener_position, utterance.source_position);
            if distance > DEFAULT_SPEECH_HEARING_RADIUS {
                continue;
            }
            let distance_gain = (1.0 - distance / DEFAULT_SPEECH_HEARING_RADIUS).clamp(0.0, 1.0);
            let confidence =
                Confidence::new((utterance.confidence.raw() * distance_gain).clamp(0.0, 1.0))?;
            for (sequence_position, token) in utterance.tokens.iter().copied().enumerate() {
                heard.push(HeardToken {
                    utterance_id: utterance.utterance_id,
                    sequence_position: u8::try_from(sequence_position)
                        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?,
                    source_kind: utterance.source_kind,
                    speaker_id: utterance.speaker_id,
                    addressee: utterance.addressee,
                    source_entity: None,
                    token_id: u32::from(token.raw()),
                    source_position: utterance.source_position,
                    confidence,
                    teacher_channel: utterance.teacher_channel,
                });
            }
        }
        heard.truncate(usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS));
        Ok(heard)
    }
}

fn distance(left: Vec3f, right: Vec3f) -> f32 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    (x * x + y * y + z * z).sqrt()
}
