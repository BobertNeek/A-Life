//! Production semantic-free physical observation extraction for grounded slots.

use std::collections::BTreeSet;

use alife_core::{
    BodySnapshot, Confidence, ContextStreams, GroundedObjectSlotV1, OrganismId, Pose,
    ScaffoldContractError, SensoryChannels, SensorySnapshot, Tick, TrackedObjectId, Validate,
    Vec3f, Velocity, WorldEntityId, MAX_GROUNDED_OBJECT_SLOTS,
};
use serde::{Deserialize, Serialize};

use crate::{
    PhysicalTrackingKey, PhysicalTrackingProvenance, StablePhysicalDescriptor,
    TrackedObjectRegistry,
};

pub const GROUNDED_VISION_RADIUS: f32 = 8.0;
pub const GROUNDED_VELOCITY_CEILING: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundedPhysicalProperties {
    pub velocity: Vec3f,
    pub color: [f32; 3],
    pub material: [f32; 3],
    pub shape: [f32; 3],
    pub chemical: [f32; 3],
    pub surface_temperature: f32,
    pub terrain: [f32; 2],
}

impl GroundedPhysicalProperties {
    pub fn deterministic_default(spawn_sequence: u64) -> Self {
        let first = splitmix64(spawn_sequence ^ 0x7151_C4A1_5EED_0001);
        let second = splitmix64(first ^ 0x91A7_EA2E_0000_0002);
        Self {
            velocity: Vec3f::ZERO,
            color: [
                unit_byte(first, 0),
                unit_byte(first, 8),
                unit_byte(first, 16),
            ],
            material: [
                unit_byte(first, 24),
                unit_byte(first, 32),
                unit_byte(first, 40),
            ],
            shape: [
                unit_byte(first, 48),
                unit_byte(first, 56),
                unit_byte(second, 0),
            ],
            chemical: [
                signed_byte(second, 8),
                signed_byte(second, 16),
                signed_byte(second, 24),
            ],
            surface_temperature: signed_byte(second, 32),
            terrain: [unit_byte(second, 40), unit_byte(second, 48)],
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.velocity.validate()?;
        if !unit_values_valid(&self.color)
            || !unit_values_valid(&self.material)
            || !unit_values_valid(&self.shape)
            || !signed_values_valid(&self.chemical)
            || !signed_value_valid(self.surface_temperature)
            || !unit_values_valid(&self.terrain)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

impl TryFrom<GroundedPhysicalProperties> for StablePhysicalDescriptor {
    type Error = ScaffoldContractError;

    fn try_from(value: GroundedPhysicalProperties) -> Result<Self, Self::Error> {
        value.validate_contract()?;
        let descriptor = Self([
            value.color[0],
            value.color[1],
            value.color[2],
            value.material[0],
            value.material[1],
            value.material[2],
            value.shape[0],
            value.shape[1],
            value.shape[2],
            value.chemical[0],
            value.chemical[1],
            value.chemical[2],
            value.surface_temperature,
            value.terrain[0],
            value.terrain[1],
        ]);
        descriptor.validate_contract()?;
        Ok(descriptor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalObservationSnapshot {
    pub observer: OrganismId,
    pub tick: Tick,
    pub observer_pose: Pose,
    pub observer_velocity: Velocity,
    pub visible: Vec<PhysicalObservedObject>,
}

impl PhysicalObservationSnapshot {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.observer.validate()?;
        self.observer_pose.validate()?;
        self.observer_velocity.validate()?;
        if self.visible.len() > MAX_GROUNDED_OBJECT_SLOTS {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut transport_ids = BTreeSet::new();
        let mut tracking_keys = BTreeSet::new();
        for object in &self.visible {
            object.validate_contract()?;
            if !transport_ids.insert(object.transport_entity.raw())
                || !tracking_keys.insert(object.tracking_key)
            {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            let relative = subtract(object.position, self.observer_pose.translation);
            if length(relative) > GROUNDED_VISION_RADIUS {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalObservedObject {
    pub transport_entity: WorldEntityId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub position: Vec3f,
    pub properties: GroundedPhysicalProperties,
    pub contact: bool,
    pub confidence: Confidence,
}

impl PhysicalObservedObject {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.transport_entity.validate()?;
        self.tracking_provenance.validate_contract()?;
        if self.tracking_key != self.tracking_provenance.canonical_key() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        self.properties.validate_contract()?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundedObjectTransport {
    pub slot_index: u16,
    pub transport_entity: WorldEntityId,
    pub target_position: Vec3f,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroundedSensingFrame {
    sensory: SensorySnapshot,
    body: BodySnapshot,
    slots: Vec<GroundedObjectSlotV1>,
    transports: Vec<GroundedObjectTransport>,
}

impl GroundedSensingFrame {
    pub fn sensory(&self) -> &SensorySnapshot {
        &self.sensory
    }

    pub const fn body(&self) -> BodySnapshot {
        self.body
    }

    pub fn slots(&self) -> &[GroundedObjectSlotV1] {
        &self.slots
    }

    pub fn transports(&self) -> &[GroundedObjectTransport] {
        &self.transports
    }

    pub fn into_parts(
        self,
    ) -> (
        SensorySnapshot,
        BodySnapshot,
        Vec<GroundedObjectSlotV1>,
        Vec<GroundedObjectTransport>,
    ) {
        (self.sensory, self.body, self.slots, self.transports)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroundedSensorExtractor;

impl GroundedSensorExtractor {
    pub fn extract(
        snapshot: &PhysicalObservationSnapshot,
        tracker: &mut TrackedObjectRegistry,
    ) -> Result<GroundedSensingFrame, ScaffoldContractError> {
        snapshot.validate_contract()?;
        if tracker.world_seed()
            != snapshot
                .visible
                .first()
                .map_or(tracker.world_seed(), |object| {
                    object.tracking_provenance.world_seed
                })
        {
            return Err(ScaffoldContractError::InvalidId);
        }

        let mut observed = snapshot.visible.to_vec();
        observed.sort_by_key(|object| object.tracking_key);
        let mut tracked = Vec::with_capacity(observed.len());
        for object in observed {
            let descriptor = StablePhysicalDescriptor::try_from(object.properties)?;
            let receipt = tracker.observe(
                snapshot.observer,
                object.tracking_provenance,
                descriptor,
                snapshot.tick,
            )?;
            let relative = subtract(object.position, snapshot.observer_pose.translation);
            tracked.push(TrackedVisibleObject {
                object,
                tracked_object_id: receipt.tracked_object_id,
                distance: length(relative),
                relative,
            });
        }
        tracked.sort_by(|left, right| {
            left.distance.total_cmp(&right.distance).then_with(|| {
                left.tracked_object_id
                    .raw()
                    .cmp(&right.tracked_object_id.raw())
            })
        });

        let linear_speed = length(snapshot.observer_velocity.linear) / GROUNDED_VELOCITY_CEILING;
        let angular_speed = length(snapshot.observer_velocity.angular) / GROUNDED_VELOCITY_CEILING;
        let mut slots = Vec::with_capacity(tracked.len());
        let mut transports = Vec::with_capacity(tracked.len());
        for (slot_index, tracked) in tracked.into_iter().enumerate() {
            let planar = tracked.relative.x.hypot(tracked.relative.y);
            let bearing = if planar > f32::EPSILON {
                [tracked.relative.y / planar, tracked.relative.x / planar]
            } else {
                [0.0, 1.0]
            };
            let relative_velocity = subtract(
                tracked.object.properties.velocity,
                snapshot.observer_velocity.linear,
            );
            let slot_index = u16::try_from(slot_index)
                .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
            let slot = GroundedObjectSlotV1 {
                slot_index,
                tracked_object_id: tracked.tracked_object_id,
                bearing,
                distance: (tracked.distance / GROUNDED_VISION_RADIUS).clamp(0.0, 1.0),
                relative_velocity: [
                    (relative_velocity.x / GROUNDED_VELOCITY_CEILING).clamp(-1.0, 1.0),
                    (relative_velocity.y / GROUNDED_VELOCITY_CEILING).clamp(-1.0, 1.0),
                    (relative_velocity.z / GROUNDED_VELOCITY_CEILING).clamp(-1.0, 1.0),
                ],
                color: tracked.object.properties.color,
                material: tracked.object.properties.material,
                shape: tracked.object.properties.shape,
                chemical: tracked.object.properties.chemical,
                contact: f32::from(tracked.object.contact),
                proprioception: [linear_speed.clamp(0.0, 1.0), angular_speed.clamp(0.0, 1.0)],
                temperature: tracked.object.properties.surface_temperature,
                terrain: tracked.object.properties.terrain,
                confidence: tracked.object.confidence,
            };
            slot.validate_contract()?;
            transports.push(GroundedObjectTransport {
                slot_index,
                transport_entity: tracked.object.transport_entity,
                target_position: tracked.object.position,
            });
            slots.push(slot);
        }

        let sensory = SensorySnapshot::new(
            snapshot.observer,
            snapshot.tick,
            snapshot.observer_pose.translation,
            SensoryChannels::ZERO,
            ContextStreams::default(),
        )?;
        Ok(GroundedSensingFrame {
            sensory,
            body: BodySnapshot {
                pose: snapshot.observer_pose,
                velocity: snapshot.observer_velocity,
            },
            slots,
            transports,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackedVisibleObject {
    object: PhysicalObservedObject,
    tracked_object_id: TrackedObjectId,
    distance: f32,
    relative: Vec3f,
}

fn signed_value_valid(value: f32) -> bool {
    value.is_finite() && (-1.0..=1.0).contains(&value)
}

fn unit_value_valid(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn signed_values_valid<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().copied().all(signed_value_valid)
}

fn unit_values_valid<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().copied().all(unit_value_valid)
}

fn subtract(left: Vec3f, right: Vec3f) -> Vec3f {
    Vec3f::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn length(value: Vec3f) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

fn unit_byte(value: u64, shift: u32) -> f32 {
    ((value >> shift) as u8) as f32 / 255.0
}

fn signed_byte(value: u64, shift: u32) -> f32 {
    unit_byte(value, shift) * 2.0 - 1.0
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
