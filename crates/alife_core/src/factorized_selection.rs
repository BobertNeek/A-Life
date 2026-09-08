use crate::{
    ActionCommand, ActionKind, ActionTarget, BoundedCoordinationSummary, BoundedMotorPayload,
    ChannelCommand, CoordinationGroup, ExperienceSequenceId, MotorChannel, MotorCommandBundle,
    OrganismId, PerceptionFrame, ScaffoldContractError, SpeechMotorPayload, Tick, Vec3f,
};

pub const VOCAL_CHANNEL_PAYLOAD_MAGIC_V1: u32 = 0x5348_5031;

/// Exact selected command contents, independent of the experience sequence
/// assigned after neural dispatch. The enclosing transaction binds sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointActionSelectionV1 {
    schema_version: u16,
    candidate_slots: [u16; 6],
    bundle_digest: [u64; 4],
}

impl JointActionSelectionV1 {
    pub fn new(
        candidate_slots: [u16; 6],
        bundle: &MotorCommandBundle,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            schema_version: 1,
            candidate_slots,
            bundle_digest: normalized_motor_bundle_digest(bundle)?,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != 1
            || self.bundle_digest == [0; 4]
            || self.candidate_slots.iter().all(|v| *v == 0)
            || self
                .candidate_slots
                .iter()
                .any(|v| usize::from(*v) > crate::MAX_ACTION_CANDIDATES)
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        for (index, value) in self.candidate_slots.iter().enumerate() {
            if *value != 0 && self.candidate_slots[..index].contains(value) {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
        }
        Ok(())
    }

    pub const fn candidate_slots(&self) -> [u16; 6] {
        self.candidate_slots
    }

    pub fn validate_bundle(
        &self,
        bundle: &MotorCommandBundle,
    ) -> Result<(), ScaffoldContractError> {
        self.validate()?;
        if normalized_motor_bundle_digest(bundle)? != self.bundle_digest {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        Ok(())
    }

    pub fn write_canonical(&self, digest: &mut crate::CanonicalDigestBuilder) {
        digest.write_u16(self.schema_version);
        for value in self.candidate_slots {
            digest.write_u16(value);
        }
        for value in self.bundle_digest {
            digest.write_u64(value);
        }
    }

    pub fn validate_decision(
        &self,
        decision: &crate::DecisionSnapshot,
    ) -> Result<(), ScaffoldContractError> {
        if let Some(bundle) = &decision.selected_bundle {
            return self.validate_bundle(bundle);
        }
        // Historical callers can omit a bundle for a single global command.
        // A genuinely joint receipt cannot match this one-command fallback.
        let bundle = arbitrate_gpu_selected_command_into_factorized_bundle(
            decision.organism_id,
            decision.sequence_id,
            decision.decision_tick,
            Vec::new(),
            &decision.selected_action,
            None,
            false,
        )?;
        self.validate_bundle(&bundle)
    }
}

pub fn normalized_motor_bundle_digest(
    bundle: &MotorCommandBundle,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut normalized = bundle.clone();
    normalized.sequence_id = ExperienceSequenceId(1);
    normalized.canonical_digest()
}

pub fn channel_command_for_action(
    channel: MotorChannel,
    command: &ActionCommand,
) -> Result<ChannelCommand, ScaffoldContractError> {
    let target = (command.target_entity.is_some() || command.target_position.is_some())
        .then(|| ActionTarget::new(command.target_entity, command.target_position));
    ChannelCommand::new(
        channel,
        command.action_id,
        target,
        command.target_position.unwrap_or(Vec3f::ZERO),
        command.intensity,
        command.duration_ticks,
        0.0,
        command.confidence,
        0,
    )
}

pub fn factorized_motor_channel_order(channel: MotorChannel) -> u16 {
    match channel {
        MotorChannel::Locomotion => 0,
        MotorChannel::Orientation => 1,
        MotorChannel::Manipulation => 2,
        MotorChannel::Vocal => 3,
        MotorChannel::Posture => 4,
        MotorChannel::SpeciesSpecific(id) => 0x100 + u16::from(id),
    }
}

pub fn arbitrate_gpu_selected_command_into_factorized_bundle(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    mut channel_commands: Vec<ChannelCommand>,
    selected_action: &ActionCommand,
    speech_payload: Option<&SpeechMotorPayload>,
    speech_prompted: bool,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    let selected_channel = match selected_action.kind {
        ActionKind::Idle
        | ActionKind::Hold
        | ActionKind::Rest
        | ActionKind::Inspect
        | ActionKind::Gesture => MotorChannel::Posture,
        ActionKind::Move => MotorChannel::Locomotion,
        ActionKind::Interact | ActionKind::Write => MotorChannel::Manipulation,
        ActionKind::Vocalize => MotorChannel::Vocal,
    };
    let mut selected = channel_command_for_action(selected_channel, selected_action)?;
    if selected_channel == MotorChannel::Vocal {
        if let Some(payload) = speech_payload {
            let mut values = Vec::with_capacity(payload.tokens.len() + 4);
            values.push(VOCAL_CHANNEL_PAYLOAD_MAGIC_V1);
            values.push(u32::from(payload.speech_act.raw()));
            values.push(if speech_prompted { 1 } else { 0 });
            values.push((payload.confidence.raw() * 65_535.0).round() as u32);
            values.extend(payload.tokens.iter().map(|token| u32::from(token.raw())));
            selected = selected.with_payload(BoundedMotorPayload::new(values)?)?;
        }
    }
    if let Some(existing) = channel_commands
        .iter_mut()
        .find(|command| command.channel == selected.channel)
    {
        *existing = selected;
    } else {
        channel_commands.push(selected);
    }
    channel_commands.sort_by_key(|command| factorized_motor_channel_order(command.channel));
    let coordination = (channel_commands.len() > 1).then(|| BoundedCoordinationSummary {
        groups: vec![CoordinationGroup {
            group_id: 0,
            channels: channel_commands
                .iter()
                .map(|command| command.channel)
                .collect(),
        }],
    });
    let bundle = MotorCommandBundle::new(organism_id, sequence_id, tick, channel_commands)?;
    if let Some(coordination) = coordination {
        bundle.with_coordination(coordination)
    } else {
        Ok(bundle)
    }
}

fn factorized_motor_channel_for_action(kind: ActionKind) -> Option<MotorChannel> {
    match kind {
        ActionKind::Move => Some(MotorChannel::Locomotion),
        ActionKind::Interact | ActionKind::Write => Some(MotorChannel::Manipulation),
        ActionKind::Vocalize => Some(MotorChannel::Vocal),
        ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => Some(MotorChannel::Posture),
        ActionKind::Idle | ActionKind::Gesture => None,
    }
}

pub fn factorized_motor_bundle_for_candidates(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    frame: &PerceptionFrame,
    candidate_slots: [u16; 6],
    channels: &[MotorChannel],
    compatibility_command: &crate::ActionCommand,
    selected_candidate_index: u16,
    speech_payload: Option<&crate::SpeechMotorPayload>,
    speech_prompted: bool,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    let mut channel_commands = Vec::with_capacity(channels.len());
    for head_channel in channels {
        let slot = match head_channel {
            MotorChannel::Locomotion => 0,
            MotorChannel::Orientation => 1,
            MotorChannel::Manipulation => 2,
            MotorChannel::Vocal => 3,
            MotorChannel::Posture => 4,
            MotorChannel::SpeciesSpecific(_) => 5,
        };
        let encoded = candidate_slots
            .get(slot)
            .copied()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if encoded == 0 || encoded - 1 == selected_candidate_index {
            continue;
        }
        let candidate_index = encoded - 1;
        let candidate = *frame
            .candidates()
            .get(usize::from(candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let channel = factorized_motor_channel_for_action(command.kind)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if channel != *head_channel {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let channel_command = channel_command_for_action(channel, &command)?;
        channel_commands.push(channel_command);
    }

    arbitrate_gpu_selected_command_into_factorized_bundle(
        organism_id,
        sequence_id,
        tick,
        channel_commands,
        compatibility_command,
        speech_payload,
        speech_prompted,
    )
}
