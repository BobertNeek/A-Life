//! Portable cognitive cores bind to embodiment through typed, replaceable ports.

use serde::{Deserialize, Serialize};

use crate::{CreaturePhenotype, ScaffoldContractError, Tick, Validate, WorldEntityId};

pub const EMBODIMENT_STATE_SCHEMA_VERSION: u16 = 3;
pub const MAX_EMBODIMENT_PORTS: usize = 32;
pub const MAX_BODY_SCHEMA_VALUES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorCapability {
    Vision,
    Hearing,
    Chemical,
    Touch,
    Proprioception,
    Interoception,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectorCapability {
    Translation,
    Rotation,
    Manipulation,
    Vocalization,
    Ingestion,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorPortDescriptor {
    pub port_id: u16,
    pub capability: SensorCapability,
    pub value_lanes: u8,
    pub sample_period_ticks: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectorPortDescriptor {
    pub port_id: u16,
    pub capability: EffectorCapability,
    pub command_lanes: u8,
    pub safety_class: u8,
}

const fn sensor_port(
    port_id: u16,
    capability: SensorCapability,
    value_lanes: u8,
) -> SensorPortDescriptor {
    SensorPortDescriptor {
        port_id,
        capability,
        value_lanes,
        sample_period_ticks: 1,
    }
}

const fn effector_port(
    port_id: u16,
    capability: EffectorCapability,
    command_lanes: u8,
    safety_class: u8,
) -> EffectorPortDescriptor {
    EffectorPortDescriptor {
        port_id,
        capability,
        command_lanes,
        safety_class,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbodimentState {
    schema_version: u16,
    adapter_id: u64,
    entity_id: WorldEntityId,
    revision: u64,
    source_tick: Tick,
    sensors: Vec<SensorPortDescriptor>,
    effectors: Vec<EffectorPortDescriptor>,
    sensor_calibration: Vec<f32>,
    effector_controllability: Vec<f32>,
    proprioceptive_body_schema: Vec<f32>,
}

impl EmbodimentState {
    pub fn from_phenotype(
        entity_id: WorldEntityId,
        source_tick: Tick,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        let sensors = vec![
            sensor_port(1, SensorCapability::Vision, 8),
            sensor_port(2, SensorCapability::Hearing, 4),
            sensor_port(3, SensorCapability::Chemical, 4),
            sensor_port(4, SensorCapability::Touch, 4),
            sensor_port(5, SensorCapability::Proprioception, 6),
            sensor_port(6, SensorCapability::Interoception, 8),
        ];
        let effectors = vec![
            effector_port(1, EffectorCapability::Translation, 3, 1),
            effector_port(2, EffectorCapability::Rotation, 3, 1),
            effector_port(3, EffectorCapability::Manipulation, 2, 1),
            effector_port(4, EffectorCapability::Vocalization, 4, 1),
            effector_port(5, EffectorCapability::Ingestion, 1, 2),
            effector_port(6, EffectorCapability::Rest, 1, 1),
        ];
        let sensor_lane_count = sensors
            .iter()
            .map(|port| usize::from(port.value_lanes))
            .sum::<usize>();
        let effector_lane_count = effectors
            .iter()
            .map(|port| usize::from(port.command_lanes))
            .sum::<usize>();
        if sensor_lane_count > MAX_BODY_SCHEMA_VALUES
            || effector_lane_count > MAX_BODY_SCHEMA_VALUES
        {
            return Err(ScaffoldContractError::InvalidGeneticBounds);
        }
        let body_schema_len = (8.0 + phenotype.body.size_scale * 16.0)
            .round()
            .clamp(8.0, MAX_BODY_SCHEMA_VALUES as f32) as usize;
        let adapter_id = phenotype.source_genome_id.0 ^ entity_id.raw().rotate_left(23);
        let sensory_calibration = (phenotype.body.sensory_acuity * 2.0 - 1.0).clamp(-1.0, 1.0);
        let effector_controllability =
            (phenotype.body.movement_efficiency * 2.0 - 1.0).clamp(-1.0, 1.0);
        let body_schema_value = (phenotype.body.size_scale * 2.0 - 1.0).clamp(-1.0, 1.0);
        let value = Self {
            schema_version: EMBODIMENT_STATE_SCHEMA_VERSION,
            adapter_id: adapter_id.max(1),
            entity_id,
            revision: 1,
            source_tick,
            sensors,
            effectors,
            sensor_calibration: vec![sensory_calibration; sensor_lane_count],
            effector_controllability: vec![effector_controllability; effector_lane_count],
            proprioceptive_body_schema: vec![body_schema_value; body_schema_len],
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn reference(
        entity_id: WorldEntityId,
        source_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            schema_version: EMBODIMENT_STATE_SCHEMA_VERSION,
            adapter_id: entity_id.raw(),
            entity_id,
            revision: 1,
            source_tick,
            sensors: vec![
                SensorPortDescriptor {
                    port_id: 1,
                    capability: SensorCapability::Vision,
                    value_lanes: 8,
                    sample_period_ticks: 1,
                },
                SensorPortDescriptor {
                    port_id: 2,
                    capability: SensorCapability::Hearing,
                    value_lanes: 4,
                    sample_period_ticks: 1,
                },
                SensorPortDescriptor {
                    port_id: 3,
                    capability: SensorCapability::Proprioception,
                    value_lanes: 6,
                    sample_period_ticks: 1,
                },
                SensorPortDescriptor {
                    port_id: 4,
                    capability: SensorCapability::Interoception,
                    value_lanes: 8,
                    sample_period_ticks: 1,
                },
            ],
            effectors: vec![
                EffectorPortDescriptor {
                    port_id: 1,
                    capability: EffectorCapability::Translation,
                    command_lanes: 3,
                    safety_class: 1,
                },
                EffectorPortDescriptor {
                    port_id: 2,
                    capability: EffectorCapability::Rotation,
                    command_lanes: 3,
                    safety_class: 1,
                },
                EffectorPortDescriptor {
                    port_id: 3,
                    capability: EffectorCapability::Vocalization,
                    command_lanes: 4,
                    safety_class: 1,
                },
                EffectorPortDescriptor {
                    port_id: 4,
                    capability: EffectorCapability::Ingestion,
                    command_lanes: 1,
                    safety_class: 2,
                },
            ],
            sensor_calibration: vec![0.0; 26],
            effector_controllability: vec![0.0; 11],
            proprioceptive_body_schema: vec![0.0; 16],
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn adapter_id(&self) -> u64 {
        self.adapter_id
    }
    pub const fn entity_id(&self) -> WorldEntityId {
        self.entity_id
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub const fn source_tick(&self) -> Tick {
        self.source_tick
    }
    pub fn sensors(&self) -> &[SensorPortDescriptor] {
        &self.sensors
    }
    pub fn effectors(&self) -> &[EffectorPortDescriptor] {
        &self.effectors
    }
    pub fn body_schema(&self) -> &[f32] {
        &self.proprioceptive_body_schema
    }
    pub fn sensor_calibration(&self) -> &[f32] {
        &self.sensor_calibration
    }
    pub fn effector_controllability(&self) -> &[f32] {
        &self.effector_controllability
    }

    pub fn sensor_gain(&self, capability: SensorCapability) -> f32 {
        let mut offset = 0_usize;
        for port in &self.sensors {
            let width = usize::from(port.value_lanes);
            if port.capability == capability {
                let values = &self.sensor_calibration[offset..offset + width];
                let mean = values.iter().copied().sum::<f32>() / width.max(1) as f32;
                return (1.0 + mean * 0.5).clamp(0.5, 1.5);
            }
            offset += width;
        }
        0.0
    }

    pub fn effector_gain(&self, capability: EffectorCapability) -> f32 {
        let mut offset = 0_usize;
        for port in &self.effectors {
            let width = usize::from(port.command_lanes);
            if port.capability == capability {
                let values = &self.effector_controllability[offset..offset + width];
                let mean = values.iter().copied().sum::<f32>() / width.max(1) as f32;
                return (1.0 + mean * 0.5).clamp(0.5, 1.5);
            }
            offset += width;
        }
        0.0
    }

    pub fn proprioceptive_gain(&self) -> f32 {
        if self.proprioceptive_body_schema.is_empty() {
            return 1.0;
        }
        let mean = self.proprioceptive_body_schema.iter().copied().sum::<f32>()
            / self.proprioceptive_body_schema.len() as f32;
        (1.0 + mean * 0.25).clamp(0.75, 1.25)
    }

    pub fn replace_calibration(
        &mut self,
        source_tick: Tick,
        sensor_calibration: Vec<f32>,
        effector_controllability: Vec<f32>,
        proprioceptive_body_schema: Vec<f32>,
    ) -> Result<(), ScaffoldContractError> {
        Tick::validate_monotonic(self.source_tick, source_tick)?;
        let original = self.clone();
        self.source_tick = source_tick;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.sensor_calibration = sensor_calibration;
        self.effector_controllability = effector_controllability;
        self.proprioceptive_body_schema = proprioceptive_body_schema;
        if let Err(error) = self.validate_contract() {
            *self = original;
            return Err(error);
        }
        Ok(())
    }
}

impl Validate for EmbodimentState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.entity_id.validate()?;
        let sensor_lane_count = self
            .sensors
            .iter()
            .map(|port| usize::from(port.value_lanes))
            .sum::<usize>();
        let effector_lane_count = self
            .effectors
            .iter()
            .map(|port| usize::from(port.command_lanes))
            .sum::<usize>();
        if self.schema_version != EMBODIMENT_STATE_SCHEMA_VERSION
            || self.adapter_id == 0
            || self.revision == 0
            || self.sensors.is_empty()
            || self.effectors.is_empty()
            || self.sensors.len() > MAX_EMBODIMENT_PORTS
            || self.effectors.len() > MAX_EMBODIMENT_PORTS
            || self.sensor_calibration.len() > MAX_BODY_SCHEMA_VALUES
            || self.effector_controllability.len() > MAX_BODY_SCHEMA_VALUES
            || self.proprioceptive_body_schema.len() > MAX_BODY_SCHEMA_VALUES
            || self.sensor_calibration.len() != sensor_lane_count
            || self.effector_controllability.len() != effector_lane_count
            || self.proprioceptive_body_schema.is_empty()
            || self.sensors.iter().any(|port| {
                port.port_id == 0 || port.value_lanes == 0 || port.sample_period_ticks == 0
            })
            || self
                .effectors
                .iter()
                .any(|port| port.port_id == 0 || port.command_lanes == 0 || port.safety_class == 0)
            || self.sensors.iter().enumerate().any(|(index, port)| {
                self.sensors[index + 1..].iter().any(|other| {
                    other.port_id == port.port_id || other.capability == port.capability
                })
            })
            || self.effectors.iter().enumerate().any(|(index, port)| {
                self.effectors[index + 1..].iter().any(|other| {
                    other.port_id == port.port_id || other.capability == port.capability
                })
            })
        {
            return Err(ScaffoldContractError::InvalidGeneticBounds);
        }
        let values = self
            .sensor_calibration
            .iter()
            .chain(&self.effector_controllability)
            .chain(&self.proprioceptive_body_schema);
        if values
            .into_iter()
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}
