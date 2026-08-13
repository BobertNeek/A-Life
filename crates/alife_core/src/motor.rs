//! Bounded factorized motor-command contracts.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, ActionTarget, CanonicalDigestBuilder, Confidence, DurationTicks,
    ExperienceSequenceId, Intensity, NormalizedScalar, OrganismId, ScaffoldContractError, Tick,
    Validate, Vec3f,
};

pub const MOTOR_COMMAND_SCHEMA_VERSION: u16 = 1;
pub const MAX_MOTOR_CHANNELS: usize = 6;
pub const MAX_MOTOR_PAYLOAD_VALUES: usize = 32;
pub const MAX_COORDINATION_GROUPS: usize = 8;

pub type TargetBinding = ActionTarget;
pub type EgocentricDirection = Vec3f;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MotorChannel {
    Locomotion,
    Orientation,
    Manipulation,
    Vocal,
    Posture,
    SpeciesSpecific(u8),
}

impl MotorChannel {
    fn canonical_key(self) -> u16 {
        match self {
            Self::Locomotion => 0,
            Self::Orientation => 1,
            Self::Manipulation => 2,
            Self::Vocal => 3,
            Self::Posture => 4,
            Self::SpeciesSpecific(id) => 0x100 + u16::from(id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedMotorPayload {
    pub values: Vec<u32>,
}

impl BoundedMotorPayload {
    pub fn new(values: Vec<u32>) -> Result<Self, ScaffoldContractError> {
        let payload = Self { values };
        payload.validate_contract()?;
        Ok(payload)
    }
}

impl Default for BoundedMotorPayload {
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

impl Validate for BoundedMotorPayload {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.values.len() > MAX_MOTOR_PAYLOAD_VALUES {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelCommand {
    pub channel: MotorChannel,
    pub primitive: ActionId,
    pub target: Option<TargetBinding>,
    pub direction: EgocentricDirection,
    pub intensity: Intensity,
    pub duration_ticks: DurationTicks,
    pub stand_off_distance: f32,
    pub payload: BoundedMotorPayload,
    pub confidence: Confidence,
    pub coordination_group: u8,
}

impl ChannelCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel: MotorChannel,
        primitive: ActionId,
        target: Option<TargetBinding>,
        direction: EgocentricDirection,
        intensity: Intensity,
        duration_ticks: DurationTicks,
        stand_off_distance: f32,
        confidence: Confidence,
        coordination_group: u8,
    ) -> Result<Self, ScaffoldContractError> {
        let command = Self {
            channel,
            primitive,
            target,
            direction,
            intensity,
            duration_ticks,
            stand_off_distance,
            payload: BoundedMotorPayload::default(),
            confidence,
            coordination_group,
        };
        command.validate_contract()?;
        Ok(command)
    }

    pub fn with_payload(
        mut self,
        payload: BoundedMotorPayload,
    ) -> Result<Self, ScaffoldContractError> {
        payload.validate_contract()?;
        self.payload = payload;
        self.validate_contract()?;
        Ok(self)
    }
}

impl Validate for ChannelCommand {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.primitive.validate()?;
        if let Some(target) = self.target {
            target.validate()?;
        }
        self.direction.validate()?;
        Intensity::new(self.intensity.raw())?;
        Confidence::new(self.confidence.raw())?;
        if self.duration_ticks.raw() == 0
            || !self.stand_off_distance.is_finite()
            || self.stand_off_distance < 0.0
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        self.payload.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationGroup {
    pub group_id: u8,
    pub channels: Vec<MotorChannel>,
}

impl Validate for CoordinationGroup {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.channels.is_empty() || self.channels.len() > MAX_MOTOR_CHANNELS {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let mut keys = self
            .channels
            .iter()
            .map(|channel| channel.canonical_key())
            .collect::<Vec<_>>();
        keys.sort_unstable();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoundedCoordinationSummary {
    pub groups: Vec<CoordinationGroup>,
}

impl Validate for BoundedCoordinationSummary {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.groups.len() > MAX_COORDINATION_GROUPS {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        for group in &self.groups {
            group.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorCommandBundle {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub tick: Tick,
    pub channels: Vec<ChannelCommand>,
    pub coordination: BoundedCoordinationSummary,
}

impl MotorCommandBundle {
    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        tick: Tick,
        channels: Vec<ChannelCommand>,
    ) -> Result<Self, ScaffoldContractError> {
        let bundle = Self {
            schema_version: MOTOR_COMMAND_SCHEMA_VERSION,
            organism_id,
            sequence_id,
            tick,
            channels,
            coordination: BoundedCoordinationSummary::default(),
        };
        bundle.validate_contract()?;
        Ok(bundle)
    }

    pub fn with_coordination(
        mut self,
        coordination: BoundedCoordinationSummary,
    ) -> Result<Self, ScaffoldContractError> {
        self.coordination = coordination;
        self.validate_contract()?;
        Ok(self)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-MOTOR-BUNDLE");
        builder.write_u16(self.schema_version);
        builder.write_u64(self.organism_id.raw());
        builder.write_u64(self.sequence_id.raw());
        builder.write_u64(self.tick.raw());
        builder.write_sequence_len(self.channels.len());
        for command in &self.channels {
            builder.write_u16(command.channel.canonical_key());
            builder.write_u32(command.primitive.raw());
            match command.target {
                Some(target) => {
                    builder.write_some();
                    write_target(&mut builder, target)?;
                }
                None => builder.write_none(),
            }
            write_vec3(&mut builder, command.direction)?;
            builder.write_f32(command.intensity.raw())?;
            builder.write_u32(command.duration_ticks.raw());
            builder.write_f32(command.stand_off_distance)?;
            builder.write_sequence_len(command.payload.values.len());
            for value in &command.payload.values {
                builder.write_u32(*value);
            }
            builder.write_f32(command.confidence.raw())?;
            builder.write_u8(command.coordination_group);
        }
        builder.write_sequence_len(self.coordination.groups.len());
        for group in &self.coordination.groups {
            builder.write_u8(group.group_id);
            builder.write_sequence_len(group.channels.len());
            for channel in &group.channels {
                builder.write_u16(channel.canonical_key());
            }
        }
        Ok(builder.finish256())
    }
}

impl Validate for MotorCommandBundle {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MOTOR_COMMAND_SCHEMA_VERSION
            || self.channels.len() > MAX_MOTOR_CHANNELS
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        let mut keys = Vec::with_capacity(self.channels.len());
        for command in &self.channels {
            command.validate_contract()?;
            keys.push(command.channel.canonical_key());
        }
        keys.sort_unstable();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        self.coordination.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeasuredChannelObservation {
    pub channel: MotorChannel,
    pub executed: bool,
    pub measured_intensity: NormalizedScalar,
    pub displacement: Vec3f,
}

impl MeasuredChannelObservation {
    pub fn new(
        channel: MotorChannel,
        executed: bool,
        measured_intensity: NormalizedScalar,
        displacement: Vec3f,
    ) -> Result<Self, ScaffoldContractError> {
        let observation = Self {
            channel,
            executed,
            measured_intensity,
            displacement,
        };
        observation.validate_contract()?;
        Ok(observation)
    }
}

impl Validate for MeasuredChannelObservation {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        NormalizedScalar::new(self.measured_intensity.raw())?;
        self.displacement.validate()?;
        Ok(())
    }
}

fn write_target(
    builder: &mut CanonicalDigestBuilder,
    target: TargetBinding,
) -> Result<(), ScaffoldContractError> {
    match target.entity {
        Some(entity) => {
            builder.write_some();
            builder.write_u64(entity.raw());
        }
        None => builder.write_none(),
    }
    match target.position {
        Some(position) => {
            builder.write_some();
            write_vec3(builder, position)?;
        }
        None => builder.write_none(),
    }
    Ok(())
}

fn write_vec3(
    builder: &mut CanonicalDigestBuilder,
    value: Vec3f,
) -> Result<(), ScaffoldContractError> {
    builder.write_f32(value.x)?;
    builder.write_f32(value.y)?;
    builder.write_f32(value.z)
}
