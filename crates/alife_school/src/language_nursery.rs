//! Perception-only language nursery using the production world speech and sensing paths.

use alife_core::{
    Confidence, HomeostaticSnapshot, LanguageTokenId, OrganismId, PerceptionFrameDraft,
    ScaffoldContractError, SensorProfile, SpeechActKind, SpeechMotorPayload,
    TeacherPerceptionChannel, Vec3f, WorldEntityId,
};
use alife_world::{
    AudibleUtterance, HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NurseryDemonstration {
    Approach,
    Eat,
    Avoid,
    Rest,
    Inspect,
    Vocalize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NurserySpeaker {
    Player {
        source_position: Vec3f,
    },
    Teacher {
        source_position: Vec3f,
    },
    Peer {
        organism_id: OrganismId,
        source_position: Vec3f,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageNurseryLesson {
    pub token: LanguageTokenId,
    pub object_label: String,
    pub object_kind: WorldObjectKind,
    pub object_position: Vec3f,
    pub demonstration: NurseryDemonstration,
}

impl LanguageNurseryLesson {
    pub fn try_new(
        token: LanguageTokenId,
        object_label: impl Into<String>,
        object_kind: WorldObjectKind,
        object_position: Vec3f,
        demonstration: NurseryDemonstration,
    ) -> Result<Self, ScaffoldContractError> {
        let object_label = object_label.into();
        if object_label.trim().is_empty()
            || object_label.chars().count() > 96
            || object_kind == WorldObjectKind::Agent
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        object_position.validate()?;
        Ok(Self {
            token,
            object_label,
            object_kind,
            object_position,
            demonstration,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageNurseryExposure {
    pub subject: OrganismId,
    pub target_entity: WorldEntityId,
    pub utterance: AudibleUtterance,
    pub perception: PerceptionFrameDraft,
    pub demonstration: NurseryDemonstration,
    pub can_issue_actions: bool,
    pub can_write_rewards: bool,
    pub can_inject_hidden_concepts: bool,
}

#[derive(Debug)]
pub struct LanguageNursery {
    subject: OrganismId,
    world: HeadlessWorld,
}

impl LanguageNursery {
    pub fn new(seed: u64, subject: OrganismId) -> Result<Self, ScaffoldContractError> {
        subject.validate()?;
        let world = HeadlessScenarioBuilder::new(seed)
            .agent("nursery-subject", subject, Vec3f::ZERO)
            .build()?;
        Ok(Self { subject, world })
    }

    pub fn world(&self) -> &HeadlessWorld {
        &self.world
    }

    pub fn present(
        &mut self,
        speaker: NurserySpeaker,
        lesson: &LanguageNurseryLesson,
    ) -> Result<LanguageNurseryExposure, ScaffoldContractError> {
        let target_entity = if let Some(existing) = self.world.entity_id(&lesson.object_label) {
            existing
        } else {
            self.world.editor_spawn_object(WorldEditorSpawnSpec {
                label: lesson.object_label.clone(),
                kind: lesson.object_kind,
                organism_id: None,
                position: lesson.object_position,
                nutrition: if lesson.object_kind == WorldObjectKind::Food {
                    1.0
                } else {
                    0.0
                },
                hazard_pain: if lesson.object_kind == WorldObjectKind::Hazard {
                    1.0
                } else {
                    0.0
                },
                radius: 0.5,
                token_id: (lesson.object_kind == WorldObjectKind::Token)
                    .then_some(u32::from(lesson.token.raw())),
            })?
        };
        let tokens = vec![lesson.token];
        let utterance = match speaker {
            NurserySpeaker::Player { source_position } => {
                self.world
                    .emit_player_tokens(Some(self.subject), source_position, tokens)?
            }
            NurserySpeaker::Teacher { source_position } => self.world.emit_teacher_tokens(
                Some(self.subject),
                source_position,
                tokens,
                TeacherPerceptionChannel::Hearing,
            )?,
            NurserySpeaker::Peer {
                organism_id,
                source_position,
            } => {
                if !self
                    .world
                    .organism_entity_ids()
                    .iter()
                    .any(|(candidate, _)| *candidate == organism_id)
                {
                    self.world.spawn_social_agent(
                        &format!("nursery-peer-{}", organism_id.raw()),
                        organism_id,
                        source_position,
                        0.5,
                    )?;
                }
                let utterance_id = alife_core::UtteranceId::new(
                    self.world
                        .audible_utterances()
                        .iter()
                        .map(|utterance| utterance.utterance_id.raw())
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1),
                )?;
                self.world.emit_creature_utterance(
                    utterance_id,
                    organism_id,
                    Some(self.subject),
                    SpeechMotorPayload::try_new(
                        SpeechActKind::Declare,
                        tokens,
                        Confidence::new(1.0)?,
                    )?,
                )?
            }
        };
        let tick = self.world.tick();
        let perception = self.world.perception_frame_draft(
            self.subject,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(tick),
        )?;
        Ok(LanguageNurseryExposure {
            subject: self.subject,
            target_entity,
            utterance,
            perception,
            demonstration: lesson.demonstration,
            can_issue_actions: false,
            can_write_rewards: false,
            can_inject_hidden_concepts: false,
        })
    }
}
