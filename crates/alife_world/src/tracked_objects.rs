//! Production world-side portable tracked-object identity.
//!
//! The registry is organism-local, deterministic, bounded, and independent of
//! engine or GPU handles. Raw world entity IDs are deliberately absent.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    CanonicalDigestBuilder, OrganismId, ScaffoldContractError, Tick, TrackedObjectId,
};
use serde::{Deserialize, Serialize};

pub const PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION: u16 = 1;
pub const DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM: u32 = 1_024;
pub const TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION: u16 = 1;

const PHYSICAL_TRACKING_DOMAIN: &[u8] = b"ALIFE-PHYSICAL-TRACK-V1";
const TRACKED_OBJECT_STATE_DOMAIN: &[u8] = b"ALIFE-TRACKED-OBJECT-STATE-V1";
const TRACKING_STREAM_SEED_A: u64 = 0xA11F_EA7E_D00D_0001;
const TRACKING_STREAM_SEED_B: u64 = 0xC0DE_CAFE_51A7_0002;
const INITIAL_TRACKED_ID_DOMAIN: u64 = 0xA11F_7A4C_0B1E_C701;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhysicalTrackingKey(pub [u64; 2]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhysicalTrackingProvenance {
    pub schema_version: u16,
    pub world_seed: u64,
    pub zone_id: u32,
    pub spawn_sequence: u64,
    pub lineage_key: u64,
}

impl PhysicalTrackingProvenance {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION
            || self.spawn_sequence == 0
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn canonical_key(&self) -> PhysicalTrackingKey {
        let mut bytes = Vec::with_capacity(PHYSICAL_TRACKING_DOMAIN.len() + 30);
        bytes.extend_from_slice(PHYSICAL_TRACKING_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        bytes.extend_from_slice(&self.world_seed.to_le_bytes());
        bytes.extend_from_slice(&self.zone_id.to_le_bytes());
        bytes.extend_from_slice(&self.spawn_sequence.to_le_bytes());
        bytes.extend_from_slice(&self.lineage_key.to_le_bytes());

        let mut first = TRACKING_STREAM_SEED_A;
        let mut second = TRACKING_STREAM_SEED_B;
        for chunk in bytes.chunks(8) {
            let mut padded = [0_u8; 8];
            padded[..chunk.len()].copy_from_slice(chunk);
            let word = u64::from_le_bytes(padded);
            first = splitmix64(first ^ word);
            second = splitmix64(second ^ word);
        }
        if first == 0 && second == 0 {
            first = splitmix64(first);
            second = splitmix64(second);
        }
        PhysicalTrackingKey([first, second])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StablePhysicalDescriptor(pub [f32; 15]);

impl StablePhysicalDescriptor {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let values = &self.0;
        if values[..9]
            .iter()
            .chain(values[13..].iter())
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || values[9..13]
                .iter()
                .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectRecord {
    pub tracked_object_id: TrackedObjectId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub last_seen_tick: Tick,
    pub stable_physical_descriptor: StablePhysicalDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectSaveRecord {
    pub tracked_object_id: TrackedObjectId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub last_seen_tick: Tick,
    pub stable_physical_descriptor: StablePhysicalDescriptor,
}

impl From<TrackedObjectRecord> for TrackedObjectSaveRecord {
    fn from(record: TrackedObjectRecord) -> Self {
        Self {
            tracked_object_id: record.tracked_object_id,
            tracking_provenance: record.tracking_provenance,
            tracking_key: record.tracking_key,
            last_seen_tick: record.last_seen_tick,
            stable_physical_descriptor: record.stable_physical_descriptor,
        }
    }
}

impl From<TrackedObjectSaveRecord> for TrackedObjectRecord {
    fn from(record: TrackedObjectSaveRecord) -> Self {
        Self {
            tracked_object_id: record.tracked_object_id,
            tracking_provenance: record.tracking_provenance,
            tracking_key: record.tracking_key,
            last_seen_tick: record.last_seen_tick,
            stable_physical_descriptor: record.stable_physical_descriptor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectRegistrySaveState {
    pub schema_version: u16,
    pub world_seed: u64,
    pub organism_id: OrganismId,
    pub capacity: u32,
    pub next_id: u64,
    pub records: Vec<TrackedObjectSaveRecord>,
}

impl TrackedObjectRegistrySaveState {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION
            || !(1..=DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM).contains(&self.capacity)
            || self.records.len() > self.capacity as usize
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.organism_id.validate()?;

        let initial_id = initial_tracked_object_id(self.world_seed, self.organism_id.raw());
        let mut previous_key = None;
        let mut seen_ids = BTreeSet::new();
        for record in &self.records {
            record.tracked_object_id.validate()?;
            record.tracking_provenance.validate_contract()?;
            record.stable_physical_descriptor.validate_contract()?;
            if record.tracking_provenance.world_seed != self.world_seed
                || record.tracking_key != record.tracking_provenance.canonical_key()
            {
                return Err(ScaffoldContractError::InvalidId);
            }
            if previous_key.is_some_and(|key| key >= record.tracking_key)
                || !seen_ids.insert(record.tracked_object_id.raw())
            {
                return Err(ScaffoldContractError::InvalidId);
            }
            if record.tracked_object_id.raw() < initial_id {
                return Err(ScaffoldContractError::MismatchedCreatureId);
            }
            previous_key = Some(record.tracking_key);
        }

        let highest_id = seen_ids
            .last()
            .copied()
            .unwrap_or(initial_id.saturating_sub(1));
        if self.next_id < initial_id || self.next_id <= highest_id {
            return Err(ScaffoldContractError::TrackedObjectIdentityExhausted);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedObjectObservationReceipt {
    pub organism_id_raw: u64,
    pub tracked_object_id: TrackedObjectId,
    pub inserted: bool,
    pub evicted: Option<TrackedObjectId>,
    pub record_count: u32,
    pub next_id: u64,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
}

#[derive(Debug, Clone)]
pub struct TrackedObjectRegistry {
    world_seed: u64,
    per_organism_capacity: u32,
    organisms: BTreeMap<u64, OrganismTrackedObjects>,
}

#[derive(Debug, Clone)]
pub struct OrganismTrackedObjects {
    organism_id_raw: u64,
    world_seed: u64,
    capacity: u32,
    next_id: u64,
    records_by_key: BTreeMap<PhysicalTrackingKey, TrackedObjectRecord>,
}

impl TrackedObjectRegistry {
    pub fn new(world_seed: u64, per_organism_capacity: u32) -> Result<Self, ScaffoldContractError> {
        if !(1..=DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM).contains(&per_organism_capacity) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            world_seed,
            per_organism_capacity,
            organisms: BTreeMap::new(),
        })
    }

    pub fn observe(
        &mut self,
        observer: OrganismId,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<TrackedObjectObservationReceipt, ScaffoldContractError> {
        observer.validate()?;
        let world_seed = self.world_seed;
        let capacity = self.per_organism_capacity;
        let observer_raw = observer.raw();
        let state = self
            .organisms
            .entry(observer_raw)
            .or_insert_with(|| OrganismTrackedObjects::new(observer_raw, world_seed, capacity));
        state.observe(provenance, descriptor, tick)
    }

    pub fn records_for(
        &self,
        observer: OrganismId,
    ) -> Option<impl ExactSizeIterator<Item = &TrackedObjectRecord>> {
        self.organisms
            .get(&observer.raw())
            .map(|state| state.records_by_key.values())
    }

    pub fn record(
        &self,
        observer: OrganismId,
        tracked_object_id: TrackedObjectId,
    ) -> Option<&TrackedObjectRecord> {
        self.organisms
            .get(&observer.raw())?
            .records_by_key
            .values()
            .find(|record| record.tracked_object_id == tracked_object_id)
    }

    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub const fn per_organism_capacity(&self) -> u32 {
        self.per_organism_capacity
    }

    pub fn save_state(
        &self,
        observer: OrganismId,
    ) -> Result<TrackedObjectRegistrySaveState, ScaffoldContractError> {
        observer.validate()?;
        let observer_raw = observer.raw();
        let state = self.organisms.get(&observer_raw);
        let saved = TrackedObjectRegistrySaveState {
            schema_version: TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION,
            world_seed: self.world_seed,
            organism_id: observer,
            capacity: self.per_organism_capacity,
            next_id: state.map_or_else(
                || initial_tracked_object_id(self.world_seed, observer_raw),
                |state| state.next_id,
            ),
            records: state.map_or_else(Vec::new, |state| {
                state
                    .records_by_key
                    .values()
                    .copied()
                    .map(TrackedObjectSaveRecord::from)
                    .collect()
            }),
        };
        saved.validate_contract()?;
        Ok(saved)
    }

    pub fn from_save_state(
        saved: TrackedObjectRegistrySaveState,
    ) -> Result<Self, ScaffoldContractError> {
        let world_seed = saved.world_seed;
        let capacity = saved.capacity;
        Self::from_save_states(world_seed, capacity, [saved])
    }

    pub fn from_save_states(
        world_seed: u64,
        per_organism_capacity: u32,
        states: impl IntoIterator<Item = TrackedObjectRegistrySaveState>,
    ) -> Result<Self, ScaffoldContractError> {
        if !(1..=DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM).contains(&per_organism_capacity) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut organisms = BTreeMap::new();
        for saved in states {
            saved.validate_contract()?;
            if saved.world_seed != world_seed || saved.capacity != per_organism_capacity {
                return Err(ScaffoldContractError::InvalidId);
            }
            let organism_id_raw = saved.organism_id.raw();
            let mut records_by_key = BTreeMap::new();
            for record in saved.records.iter().copied() {
                records_by_key.insert(record.tracking_key, TrackedObjectRecord::from(record));
            }
            let state = OrganismTrackedObjects {
                organism_id_raw,
                world_seed: saved.world_seed,
                capacity: saved.capacity,
                next_id: saved.next_id,
                records_by_key,
            };
            if organisms.insert(organism_id_raw, state).is_some() {
                return Err(ScaffoldContractError::MismatchedCreatureId);
            }
        }
        Ok(Self {
            world_seed,
            per_organism_capacity,
            organisms,
        })
    }
}

impl OrganismTrackedObjects {
    fn new(organism_id_raw: u64, world_seed: u64, capacity: u32) -> Self {
        Self {
            organism_id_raw,
            world_seed,
            capacity,
            next_id: initial_tracked_object_id(world_seed, organism_id_raw),
            records_by_key: BTreeMap::new(),
        }
    }

    fn observe(
        &mut self,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<TrackedObjectObservationReceipt, ScaffoldContractError> {
        provenance.validate_contract()?;
        descriptor.validate_contract()?;
        if provenance.world_seed != self.world_seed {
            return Err(ScaffoldContractError::InvalidId);
        }
        let tracking_key = provenance.canonical_key();
        let before_digest = self.canonical_digest();

        if let Some(existing) = self.records_by_key.get_mut(&tracking_key) {
            if tick.raw() < existing.last_seen_tick.raw() {
                return Err(ScaffoldContractError::NonMonotonicTick);
            }
            existing.last_seen_tick = tick;
            existing.stable_physical_descriptor = descriptor;
            let tracked_object_id = existing.tracked_object_id;
            return Ok(TrackedObjectObservationReceipt {
                organism_id_raw: self.organism_id_raw,
                tracked_object_id,
                inserted: false,
                evicted: None,
                record_count: self.records_by_key.len() as u32,
                next_id: self.next_id,
                before_digest,
                after_digest: self.canonical_digest(),
            });
        }

        let tracked_object_id = TrackedObjectId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::TrackedObjectIdentityExhausted)?;

        let evicted = if self.records_by_key.len() == self.capacity as usize {
            let key = self
                .records_by_key
                .iter()
                .min_by_key(|(key, record)| {
                    (
                        record.last_seen_tick.raw(),
                        record.tracked_object_id.raw(),
                        **key,
                    )
                })
                .map(|(key, _)| *key)
                .expect("positive capacity has an eviction target");
            self.records_by_key
                .remove(&key)
                .map(|record| record.tracked_object_id)
        } else {
            None
        };

        self.records_by_key.insert(
            tracking_key,
            TrackedObjectRecord {
                tracked_object_id,
                tracking_provenance: provenance,
                tracking_key,
                last_seen_tick: tick,
                stable_physical_descriptor: descriptor,
            },
        );
        Ok(TrackedObjectObservationReceipt {
            organism_id_raw: self.organism_id_raw,
            tracked_object_id,
            inserted: true,
            evicted,
            record_count: self.records_by_key.len() as u32,
            next_id: self.next_id,
            before_digest,
            after_digest: self.canonical_digest(),
        })
    }

    fn canonical_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(TRACKED_OBJECT_STATE_DOMAIN);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.world_seed);
        digest.write_u32(self.capacity);
        digest.write_u64(self.next_id);
        digest.write_sequence_len(self.records_by_key.len());
        for (key, record) in &self.records_by_key {
            digest.write_u64(key.0[0]);
            digest.write_u64(key.0[1]);
            digest.write_u64(record.tracked_object_id.raw());
            digest.write_u16(record.tracking_provenance.schema_version);
            digest.write_u64(record.tracking_provenance.world_seed);
            digest.write_u32(record.tracking_provenance.zone_id);
            digest.write_u64(record.tracking_provenance.spawn_sequence);
            digest.write_u64(record.tracking_provenance.lineage_key);
            digest.write_u64(record.last_seen_tick.raw());
            for value in record.stable_physical_descriptor.0 {
                digest
                    .write_f32(value)
                    .expect("validated tracked descriptors are finite");
            }
        }
        digest.finish256()
    }
}

pub fn initial_tracked_object_id(world_seed: u64, organism_id_raw: u64) -> u64 {
    1 + (splitmix64(world_seed ^ organism_id_raw.rotate_left(23) ^ INITIAL_TRACKED_ID_DOMAIN)
        & 0x0000_FFFF_FFFF_FFFF)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
