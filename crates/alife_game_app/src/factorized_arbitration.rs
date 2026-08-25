use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BoundedCoordinationSummary, BoundedMotorPayload,
    ChannelCommand, CoordinationGroup, ExperienceSequenceId, MotorChannel, MotorCommandBundle,
    OrganismId, ScaffoldContractError, SpeechMotorPayload, Tick, Vec3f,
};

pub(crate) const VOCAL_CHANNEL_PAYLOAD_MAGIC_V1: u32 = 0x5348_5031;

pub(crate) fn channel_command_for_action(
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

pub(crate) fn factorized_motor_channel_order(channel: MotorChannel) -> u16 {
    match channel {
        MotorChannel::Locomotion => 0,
        MotorChannel::Orientation => 1,
        MotorChannel::Manipulation => 2,
        MotorChannel::Vocal => 3,
        MotorChannel::Posture => 4,
        MotorChannel::SpeciesSpecific(id) => 0x100 + u16::from(id),
    }
}

pub(crate) fn arbitrate_gpu_selected_command_into_factorized_bundle(
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
