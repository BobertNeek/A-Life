//! G07 deterministic ecology/resource-cycle contracts for headless worlds.
//!
//! This module stays Bevy-independent. It models terrain zones, bounded
//! resource regrowth/spawn policies, hazard pressure, and compact ecology
//! metrics for tests and product smoke paths.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{NormalizedScalar, ScaffoldContractError, Tick, Vec3f, WorldEntityId};

pub const G07_ECOLOGY_SCHEMA: &str = "alife.g07.world_ecology.v1";
pub const G07_ECOLOGY_SCHEMA_VERSION: u16 = 1;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EcologyZoneId(pub u32);

impl EcologyZoneId {
    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.0 == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(())
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum TerrainZoneKind {
    Meadow,
    Grove,
    Wetland,
    Rocky,
    Nest,
    HazardField,
}

impl TerrainZoneKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Meadow => "meadow",
            Self::Grove => "grove",
            Self::Wetland => "wetland",
            Self::Rocky => "rocky",
            Self::Nest => "nest",
            Self::HazardField => "hazard-field",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TerrainZone {
    pub id: EcologyZoneId,
    pub label: String,
    pub kind: TerrainZoneKind,
    pub center: Vec3f,
    pub radius: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl TerrainZone {
    pub fn new(
        id: EcologyZoneId,
        label: impl Into<String>,
        kind: TerrainZoneKind,
        center: Vec3f,
        radius: f32,
        resource_bias: f32,
        hazard_pressure: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let zone = Self {
            id,
            label: label.into(),
            kind,
            center,
            radius,
            resource_bias,
            hazard_pressure,
        };
        zone.validate()?;
        Ok(zone)
    }

    pub fn contains(&self, position: Vec3f) -> bool {
        distance(self.center, position) <= self.radius
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.center.validate()?;
        for value in [self.radius, self.resource_bias, self.hazard_pressure] {
            if !value.is_finite() {
                return Err(ScaffoldContractError::NonFiniteFloat);
            }
        }
        if self.radius <= 0.0
            || !(0.0..=1.0).contains(&self.resource_bias)
            || !(0.0..=1.0).contains(&self.hazard_pressure)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceLifecycle {
    pub object_id: WorldEntityId,
    pub home_zone: EcologyZoneId,
    pub base_nutrition: f32,
    pub regrow_after_ticks: u32,
    pub decay_after_ticks: u32,
    pub consumed_at_tick: Option<Tick>,
    pub last_regrown_tick: Option<Tick>,
    pub low_salience_marker: bool,
}

impl ResourceLifecycle {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.object_id.validate()?;
        self.home_zone.validate()?;
        NormalizedScalar::new(self.base_nutrition)?;
        if self.regrow_after_ticks == 0 || self.decay_after_ticks == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceSpawnPolicy {
    pub label_prefix: String,
    pub zone_id: EcologyZoneId,
    pub interval_ticks: u32,
    pub max_active: usize,
    pub nutrition: f32,
    pub next_spawn_tick: Tick,
    pub spawned_count: u32,
}

impl ResourceSpawnPolicy {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.zone_id.validate()?;
        if self.label_prefix.is_empty() || self.interval_ticks == 0 || self.max_active == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.nutrition)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EcologyConfig {
    pub max_world_objects: usize,
    pub max_resource_records: usize,
    pub max_zones: usize,
    pub max_spawn_per_tick: usize,
    pub cycle_length_ticks: u32,
}

impl Default for EcologyConfig {
    fn default() -> Self {
        Self {
            max_world_objects: 64,
            max_resource_records: 32,
            max_zones: 8,
            max_spawn_per_tick: 2,
            cycle_length_ticks: 24,
        }
    }
}

impl EcologyConfig {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.max_world_objects == 0
            || self.max_resource_records == 0
            || self.max_zones == 0
            || self.max_spawn_per_tick == 0
            || self.cycle_length_ticks == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EcologyMetrics {
    pub active_resources: usize,
    pub consumed_resources: usize,
    pub active_hazards: usize,
    pub zones: usize,
    pub resources_regrown: u32,
    pub resources_spawned: u32,
    pub resources_decayed: u32,
    pub cleanup_marked: u32,
    pub cap_rejections: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EcologyStepReport {
    pub tick: Tick,
    pub regrown_entities: Vec<WorldEntityId>,
    pub spawned_labels: Vec<String>,
    pub decayed_entities: Vec<WorldEntityId>,
    pub cleanup_marked_entities: Vec<WorldEntityId>,
    pub cap_rejections: u32,
    pub metrics: EcologyMetrics,
}

impl Default for EcologyStepReport {
    fn default() -> Self {
        Self {
            tick: Tick::ZERO,
            regrown_entities: Vec::new(),
            spawned_labels: Vec::new(),
            decayed_entities: Vec::new(),
            cleanup_marked_entities: Vec::new(),
            cap_rejections: 0,
            metrics: EcologyMetrics::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EcologySensorySummary {
    pub schema: String,
    pub schema_version: u16,
    pub current_zone: Option<EcologyZoneId>,
    pub terrain_kind: Option<TerrainZoneKind>,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub cycle_phase: f32,
    pub active_resources: usize,
    pub active_hazards: usize,
}

impl EcologySensorySummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G07_ECOLOGY_SCHEMA || self.schema_version != G07_ECOLOGY_SCHEMA_VERSION {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(id) = self.current_zone {
            id.validate()?;
        }
        for value in [self.resource_bias, self.hazard_pressure, self.cycle_phase] {
            NormalizedScalar::new(value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EcologyState {
    pub schema: String,
    pub schema_version: u16,
    pub config: EcologyConfig,
    pub zones: Vec<TerrainZone>,
    pub resources: Vec<ResourceLifecycle>,
    pub spawn_policies: Vec<ResourceSpawnPolicy>,
    pub metrics: EcologyMetrics,
}

impl Default for EcologyState {
    fn default() -> Self {
        Self {
            schema: G07_ECOLOGY_SCHEMA.to_string(),
            schema_version: G07_ECOLOGY_SCHEMA_VERSION,
            config: EcologyConfig::default(),
            zones: Vec::new(),
            resources: Vec::new(),
            spawn_policies: Vec::new(),
            metrics: EcologyMetrics::default(),
        }
    }
}

impl EcologyState {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G07_ECOLOGY_SCHEMA || self.schema_version != G07_ECOLOGY_SCHEMA_VERSION {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.config.validate()?;
        if self.zones.len() > self.config.max_zones
            || self.resources.len() > self.config.max_resource_records
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut zone_ids = BTreeSet::new();
        for zone in &self.zones {
            zone.validate()?;
            if !zone_ids.insert(zone.id.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        let mut object_ids = BTreeSet::new();
        for resource in &self.resources {
            resource.validate()?;
            if !zone_ids.contains(&resource.home_zone.raw())
                || !object_ids.insert(resource.object_id.raw())
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        for policy in &self.spawn_policies {
            policy.validate()?;
            if !zone_ids.contains(&policy.zone_id.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn with_config(mut self, config: EcologyConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        self.config = config;
        self.validate()?;
        Ok(self)
    }

    pub fn add_zone(&mut self, zone: TerrainZone) -> Result<(), ScaffoldContractError> {
        zone.validate()?;
        if self.zones.len() >= self.config.max_zones
            || self.zones.iter().any(|existing| existing.id == zone.id)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.zones.push(zone);
        self.zones.sort_by_key(|zone| zone.id.raw());
        Ok(())
    }

    pub fn add_resource(
        &mut self,
        resource: ResourceLifecycle,
    ) -> Result<(), ScaffoldContractError> {
        resource.validate()?;
        if self.resources.len() >= self.config.max_resource_records
            || self
                .resources
                .iter()
                .any(|existing| existing.object_id == resource.object_id)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.resources.push(resource);
        self.resources
            .sort_by_key(|resource| resource.object_id.raw());
        Ok(())
    }

    pub fn add_spawn_policy(
        &mut self,
        policy: ResourceSpawnPolicy,
    ) -> Result<(), ScaffoldContractError> {
        policy.validate()?;
        self.spawn_policies.push(policy);
        self.spawn_policies.sort_by(|a, b| {
            a.zone_id
                .cmp(&b.zone_id)
                .then(a.label_prefix.cmp(&b.label_prefix))
        });
        Ok(())
    }

    pub fn record_consumed(&mut self, object_id: WorldEntityId, tick: Tick) {
        if let Some(resource) = self
            .resources
            .iter_mut()
            .find(|resource| resource.object_id == object_id)
        {
            resource.consumed_at_tick = Some(tick);
            resource.low_salience_marker = true;
        }
    }

    pub fn zone_at(&self, position: Vec3f) -> Option<&TerrainZone> {
        self.zones
            .iter()
            .filter(|zone| zone.contains(position))
            .max_by(|a, b| {
                a.hazard_pressure
                    .total_cmp(&b.hazard_pressure)
                    .then(a.resource_bias.total_cmp(&b.resource_bias))
                    .then_with(|| b.id.raw().cmp(&a.id.raw()))
            })
    }

    pub fn sensory_summary(
        &self,
        position: Vec3f,
        tick: Tick,
        active_resources: usize,
        active_hazards: usize,
    ) -> EcologySensorySummary {
        let zone = self.zone_at(position);
        EcologySensorySummary {
            schema: G07_ECOLOGY_SCHEMA.to_string(),
            schema_version: G07_ECOLOGY_SCHEMA_VERSION,
            current_zone: zone.map(|zone| zone.id),
            terrain_kind: zone.map(|zone| zone.kind),
            resource_bias: zone.map_or(0.0, |zone| zone.resource_bias),
            hazard_pressure: zone.map_or(0.0, |zone| zone.hazard_pressure),
            cycle_phase: cycle_phase(tick, self.config.cycle_length_ticks),
            active_resources,
            active_hazards,
        }
    }

    pub fn metrics(&self) -> EcologyMetrics {
        self.metrics.clone()
    }

    pub(crate) fn rebuild_metrics(&mut self, object_kinds: &BTreeMap<u64, (bool, bool)>) {
        self.metrics.active_resources = object_kinds
            .values()
            .filter(|(is_food, consumed)| *is_food && !*consumed)
            .count();
        self.metrics.consumed_resources = object_kinds
            .values()
            .filter(|(is_food, consumed)| *is_food && *consumed)
            .count();
        self.metrics.active_hazards = object_kinds
            .values()
            .filter(|(is_food, consumed)| !*is_food && !*consumed)
            .count();
        self.metrics.zones = self.zones.len();
    }

    pub(crate) fn resource_by_object_mut(
        &mut self,
        object_id: WorldEntityId,
    ) -> Option<&mut ResourceLifecycle> {
        self.resources
            .iter_mut()
            .find(|resource| resource.object_id == object_id)
    }

    pub(crate) fn zone(&self, id: EcologyZoneId) -> Option<&TerrainZone> {
        self.zones.iter().find(|zone| zone.id == id)
    }
}

pub fn cycle_phase(tick: Tick, cycle_length_ticks: u32) -> f32 {
    if cycle_length_ticks == 0 {
        return 0.0;
    }
    (tick.raw() % cycle_length_ticks as u64) as f32 / cycle_length_ticks as f32
}

pub fn deterministic_zone_position(zone: &TerrainZone, index: u32) -> Vec3f {
    let radius = zone.radius * 0.45;
    let slot = index % 8;
    let (dx, dy) = match slot {
        0 => (0.0, 0.0),
        1 => (radius, 0.0),
        2 => (-radius, 0.0),
        3 => (0.0, radius),
        4 => (0.0, -radius),
        5 => (radius * 0.7, radius * 0.7),
        6 => (-radius * 0.7, radius * 0.7),
        _ => (radius * 0.7, -radius * 0.7),
    };
    Vec3f::new(zone.center.x + dx, zone.center.y + dy, zone.center.z)
}

fn distance(a: Vec3f, b: Vec3f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
