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
        const DOMAIN_LEN: usize = PHYSICAL_TRACKING_DOMAIN.len();
        const KEY_LEN: usize = DOMAIN_LEN + 30;
        const _: () = assert!(KEY_LEN == 53);

        let mut bytes = [0_u8; KEY_LEN];
        bytes[..DOMAIN_LEN].copy_from_slice(PHYSICAL_TRACKING_DOMAIN);
        bytes[DOMAIN_LEN..DOMAIN_LEN + 2].copy_from_slice(&self.schema_version.to_le_bytes());
        bytes[DOMAIN_LEN + 2..DOMAIN_LEN + 10].copy_from_slice(&self.world_seed.to_le_bytes());
        bytes[DOMAIN_LEN + 10..DOMAIN_LEN + 14].copy_from_slice(&self.zone_id.to_le_bytes());
        bytes[DOMAIN_LEN + 14..DOMAIN_LEN + 22].copy_from_slice(&self.spawn_sequence.to_le_bytes());
        bytes[DOMAIN_LEN + 22..KEY_LEN].copy_from_slice(&self.lineage_key.to_le_bytes());

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

pub type TrackedObjectSaveRecord = TrackedObjectRecord;

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

#[derive(Debug)]
enum TrackedObjectTxUndo {
    ObserveExisting {
        observer_raw: u64,
        previous_next_id: u64,
        ops: Vec<BatchUndoOp>,
    },
    RemoveInsertedOrganism {
        observer_raw: u64,
    },
    ReplaceRegistry {
        world_seed: u64,
        per_organism_capacity: u32,
        organisms: BTreeMap<u64, OrganismTrackedObjects>,
    },
}

#[derive(Debug)]
pub struct TrackedObjectRegistry {
    world_seed: u64,
    per_organism_capacity: u32,
    organisms: BTreeMap<u64, OrganismTrackedObjects>,
    staged_tx: Option<Vec<TrackedObjectTxUndo>>,
}

impl Clone for TrackedObjectRegistry {
    fn clone(&self) -> Self {
        Self {
            world_seed: self.world_seed,
            per_organism_capacity: self.per_organism_capacity,
            organisms: self.organisms.clone(),
            staged_tx: None,
        }
    }
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
            staged_tx: None,
        })
    }

    pub(crate) fn empty_placeholder(&self) -> Self {
        Self {
            world_seed: self.world_seed,
            per_organism_capacity: self.per_organism_capacity,
            organisms: BTreeMap::new(),
            staged_tx: None,
        }
    }

    pub(crate) fn begin_staged_transaction(&mut self) {
        self.staged_tx = Some(Vec::new());
    }

    pub(crate) fn commit_staged_transaction(&mut self) {
        self.staged_tx = None;
    }

    pub(crate) fn rollback_staged_transaction(&mut self) {
        if let Some(ops) = self.staged_tx.take() {
            for op in ops.into_iter().rev() {
                match op {
                    TrackedObjectTxUndo::ObserveExisting {
                        observer_raw,
                        previous_next_id,
                        ops,
                    } => {
                        let existing = self
                            .organisms
                            .get_mut(&observer_raw)
                            .expect("staged observer exists during rollback");
                        for batch_op in ops.into_iter().rev() {
                            existing.apply_undo(batch_op);
                        }
                        existing.next_id = previous_next_id;
                    }
                    TrackedObjectTxUndo::RemoveInsertedOrganism { observer_raw } => {
                        self.organisms.remove(&observer_raw);
                    }
                    TrackedObjectTxUndo::ReplaceRegistry {
                        world_seed,
                        per_organism_capacity,
                        organisms,
                    } => {
                        self.world_seed = world_seed;
                        self.per_organism_capacity = per_organism_capacity;
                        self.organisms = organisms;
                    }
                }
            }
        }
    }

    pub(crate) fn adopt_staged_transaction_from(&mut self, prior: &mut Self) {
        self.staged_tx = prior.staged_tx.take();
    }

    pub(crate) fn replace_with_staged_backup(&mut self, mut replacement: Self) {
        if let Some(mut tx) = self.staged_tx.take() {
            tx.push(TrackedObjectTxUndo::ReplaceRegistry {
                world_seed: self.world_seed,
                per_organism_capacity: self.per_organism_capacity,
                organisms: std::mem::take(&mut self.organisms),
            });
            replacement.staged_tx = Some(tx);
        }
        *self = replacement;
    }

    pub fn observe(
        &mut self,
        observer: OrganismId,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<TrackedObjectObservationReceipt, ScaffoldContractError> {
        observer.validate()?;
        let observer_raw = observer.raw();
        if let Some(existing) = self.organisms.get_mut(&observer_raw) {
            let before_digest = existing.canonical_digest();
            let previous_next_id = existing.next_id;
            let (tracked_object_id, op) =
                existing.observe_with_undo(provenance, descriptor, tick)?;
            let inserted = matches!(op, BatchUndoOp::RevertInsert { .. });
            let evicted = match &op {
                BatchUndoOp::RevertInsert { evicted, .. } => {
                    evicted.map(|record| record.tracked_object_id)
                }
                BatchUndoOp::RestoreExisting(_) => None,
            };
            let receipt = TrackedObjectObservationReceipt {
                organism_id_raw: observer_raw,
                tracked_object_id,
                inserted,
                evicted,
                record_count: existing.records_by_key.len() as u32,
                next_id: existing.next_id,
                before_digest,
                after_digest: existing.canonical_digest(),
            };
            if let Some(tx) = self.staged_tx.as_mut() {
                tx.push(TrackedObjectTxUndo::ObserveExisting {
                    observer_raw,
                    previous_next_id,
                    ops: vec![op],
                });
            }
            return Ok(receipt);
        }

        let mut candidate =
            OrganismTrackedObjects::new(observer_raw, self.world_seed, self.per_organism_capacity);
        let before_digest = candidate.canonical_digest();
        let (tracked_object_id, inserted, evicted) =
            candidate.observe(provenance, descriptor, tick)?;
        let receipt = TrackedObjectObservationReceipt {
            organism_id_raw: observer_raw,
            tracked_object_id,
            inserted,
            evicted,
            record_count: candidate.records_by_key.len() as u32,
            next_id: candidate.next_id,
            before_digest,
            after_digest: candidate.canonical_digest(),
        };
        self.organisms.insert(observer_raw, candidate);
        if let Some(tx) = self.staged_tx.as_mut() {
            tx.push(TrackedObjectTxUndo::RemoveInsertedOrganism { observer_raw });
        }
        Ok(receipt)
    }

    pub(crate) fn observe_batch(
        &mut self,
        observer: OrganismId,
        observations: impl IntoIterator<
            Item = (PhysicalTrackingProvenance, StablePhysicalDescriptor, Tick),
        >,
    ) -> Result<Vec<TrackedObjectId>, ScaffoldContractError> {
        observer.validate()?;
        if let Some(existing) = self.organisms.get_mut(&observer.raw()) {
            let original_next_id = existing.next_id;
            let mut undo = Vec::new();
            let mut ids = Vec::new();
            for (provenance, descriptor, tick) in observations {
                match existing.observe_with_undo(provenance, descriptor, tick) {
                    Ok((id, op)) => {
                        undo.push(op);
                        ids.push(id);
                    }
                    Err(error) => {
                        for op in undo.into_iter().rev() {
                            existing.apply_undo(op);
                        }
                        existing.next_id = original_next_id;
                        return Err(error);
                    }
                }
            }
            if let Some(tx) = self.staged_tx.as_mut() {
                if !undo.is_empty() {
                    tx.push(TrackedObjectTxUndo::ObserveExisting {
                        observer_raw: observer.raw(),
                        previous_next_id: original_next_id,
                        ops: undo,
                    });
                }
            }
            return Ok(ids);
        }

        let mut candidate = OrganismTrackedObjects::new(
            observer.raw(),
            self.world_seed,
            self.per_organism_capacity,
        );
        let mut ids = Vec::new();
        for (provenance, descriptor, tick) in observations {
            ids.push(candidate.observe(provenance, descriptor, tick)?.0);
        }
        if !ids.is_empty() {
            self.organisms.insert(observer.raw(), candidate);
            if let Some(tx) = self.staged_tx.as_mut() {
                tx.push(TrackedObjectTxUndo::RemoveInsertedOrganism {
                    observer_raw: observer.raw(),
                });
            }
        }
        Ok(ids)
    }

    pub fn records_for(
        &self,
        observer: OrganismId,
    ) -> Option<impl ExactSizeIterator<Item = &TrackedObjectRecord>> {
        self.organisms
            .get(&observer.raw())
            .map(|state| state.records_by_key.values())
    }

    pub fn record_by_key(
        &self,
        observer: OrganismId,
        tracking_key: PhysicalTrackingKey,
    ) -> Option<&TrackedObjectRecord> {
        self.organisms
            .get(&observer.raw())?
            .records_by_key
            .get(&tracking_key)
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
                state.records_by_key.values().copied().collect()
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
                records_by_key.insert(record.tracking_key, record);
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
            staged_tx: None,
        })
    }
}

#[derive(Debug)]
enum BatchUndoOp {
    RestoreExisting(TrackedObjectRecord),
    RevertInsert {
        inserted_key: PhysicalTrackingKey,
        evicted: Option<TrackedObjectRecord>,
    },
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

    fn apply_undo(&mut self, op: BatchUndoOp) {
        match op {
            BatchUndoOp::RestoreExisting(record) => {
                self.records_by_key.insert(record.tracking_key, record);
            }
            BatchUndoOp::RevertInsert {
                inserted_key,
                evicted,
            } => {
                self.records_by_key.remove(&inserted_key);
                if let Some(evicted_record) = evicted {
                    self.records_by_key
                        .insert(evicted_record.tracking_key, evicted_record);
                }
            }
        }
    }

    fn observe_with_undo(
        &mut self,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<(TrackedObjectId, BatchUndoOp), ScaffoldContractError> {
        provenance.validate_contract()?;
        descriptor.validate_contract()?;
        if provenance.world_seed != self.world_seed {
            return Err(ScaffoldContractError::InvalidId);
        }
        let tracking_key = provenance.canonical_key();

        if let Some(existing) = self.records_by_key.get_mut(&tracking_key) {
            if tick.raw() < existing.last_seen_tick.raw() {
                return Err(ScaffoldContractError::NonMonotonicTick);
            }
            let previous = *existing;
            existing.last_seen_tick = tick;
            existing.stable_physical_descriptor = descriptor;
            return Ok((
                existing.tracked_object_id,
                BatchUndoOp::RestoreExisting(previous),
            ));
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
            self.records_by_key.remove(&key)
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
        Ok((
            tracked_object_id,
            BatchUndoOp::RevertInsert {
                inserted_key: tracking_key,
                evicted,
            },
        ))
    }

    fn observe(
        &mut self,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<(TrackedObjectId, bool, Option<TrackedObjectId>), ScaffoldContractError> {
        provenance.validate_contract()?;
        descriptor.validate_contract()?;
        if provenance.world_seed != self.world_seed {
            return Err(ScaffoldContractError::InvalidId);
        }
        let tracking_key = provenance.canonical_key();

        if let Some(existing) = self.records_by_key.get_mut(&tracking_key) {
            if tick.raw() < existing.last_seen_tick.raw() {
                return Err(ScaffoldContractError::NonMonotonicTick);
            }
            existing.last_seen_tick = tick;
            existing.stable_physical_descriptor = descriptor;
            let tracked_object_id = existing.tracked_object_id;
            return Ok((tracked_object_id, false, None));
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
        Ok((tracked_object_id, true, evicted))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance(spawn_sequence: u64) -> PhysicalTrackingProvenance {
        PhysicalTrackingProvenance {
            schema_version: PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION,
            world_seed: 77,
            zone_id: 0,
            spawn_sequence,
            lineage_key: 0,
        }
    }

    fn descriptor() -> StablePhysicalDescriptor {
        StablePhysicalDescriptor([0.0; 15])
    }

    #[test]
    fn failed_observation_batch_preserves_the_observer_registry() {
        let observer = OrganismId(1);
        let mut registry = TrackedObjectRegistry::new(77, 4).unwrap();
        registry
            .observe(observer, provenance(2), descriptor(), Tick::new(10))
            .unwrap();
        let before = registry.save_state(observer).unwrap();

        assert!(registry
            .observe_batch(
                observer,
                [
                    (provenance(1), descriptor(), Tick::new(5)),
                    (provenance(2), descriptor(), Tick::new(9)),
                ],
            )
            .is_err());
        assert_eq!(registry.save_state(observer).unwrap(), before);
    }

    #[test]
    fn sensing_batch_preserves_audited_ids_eviction_and_restore_continuation() {
        let owner = OrganismId(1);
        let mut audited = TrackedObjectRegistry::new(77, 2).unwrap();
        let mut sensing = audited.clone();
        for batch in [
            vec![
                (provenance(1), descriptor(), Tick::new(1)),
                (provenance(2), descriptor(), Tick::new(2)),
            ],
            vec![
                (provenance(1), descriptor(), Tick::new(3)),
                (provenance(3), descriptor(), Tick::new(4)),
            ],
            vec![(provenance(2), descriptor(), Tick::new(5))],
        ] {
            let mut expected_ids = Vec::new();
            for (provenance, descriptor, tick) in batch.iter().copied() {
                let receipt = audited
                    .observe(owner, provenance, descriptor, tick)
                    .unwrap();
                assert_ne!(receipt.before_digest, receipt.after_digest);
                expected_ids.push(receipt.tracked_object_id);
            }
            assert_eq!(sensing.observe_batch(owner, batch).unwrap(), expected_ids);
            let saved = sensing.save_state(owner).unwrap();
            assert_eq!(saved, audited.save_state(owner).unwrap());
            sensing = TrackedObjectRegistry::from_save_state(saved).unwrap();
        }
    }

    #[test]
    fn record_by_key_and_evicting_batch_rollback_preserve_exact_state() {
        let observer = OrganismId(1);
        let mut registry = TrackedObjectRegistry::new(77, 2).unwrap();
        registry
            .observe_batch(
                observer,
                [
                    (provenance(1), descriptor(), Tick::new(1)),
                    (provenance(2), descriptor(), Tick::new(2)),
                ],
            )
            .unwrap();
        let key1 = provenance(1).canonical_key();
        let rec1 = registry.record_by_key(observer, key1).unwrap();
        assert_eq!(rec1.tracking_key, key1);
        assert_eq!(rec1.last_seen_tick, Tick::new(1));
        assert!(registry
            .record_by_key(observer, provenance(99).canonical_key())
            .is_none());

        let before = registry.save_state(observer).unwrap();
        // Batch updates existing key, inserts new key causing eviction, updates same key again,
        // and then fails on non-monotonic tick for provenance(2).
        let err = registry
            .observe_batch(
                observer,
                [
                    (
                        provenance(2),
                        StablePhysicalDescriptor([0.25; 15]),
                        Tick::new(5),
                    ),
                    (provenance(3), descriptor(), Tick::new(6)),
                    (
                        provenance(3),
                        StablePhysicalDescriptor([0.5; 15]),
                        Tick::new(7),
                    ),
                    (provenance(2), descriptor(), Tick::new(4)),
                ],
            )
            .unwrap_err();
        assert_eq!(err, ScaffoldContractError::NonMonotonicTick);
        assert_eq!(registry.save_state(observer).unwrap(), before);
    }

    #[test]
    fn staged_transaction_rollback_restores_new_updated_and_evicted_records_exactly() {
        let observer_a = OrganismId(1);
        let observer_b = OrganismId(2);
        let mut registry = TrackedObjectRegistry::new(77, 2).unwrap();
        registry
            .observe_batch(
                observer_a,
                [
                    (provenance(1), descriptor(), Tick::new(1)),
                    (provenance(2), descriptor(), Tick::new(2)),
                ],
            )
            .unwrap();
        let before_a = registry.save_state(observer_a).unwrap();
        let before_b = registry.save_state(observer_b).unwrap();

        registry.begin_staged_transaction();
        registry
            .observe_batch(
                observer_a,
                [
                    (
                        provenance(1),
                        StablePhysicalDescriptor([0.75; 15]),
                        Tick::new(5),
                    ),
                    (provenance(3), descriptor(), Tick::new(6)),
                ],
            )
            .unwrap();
        registry
            .observe(
                observer_b,
                provenance(10),
                StablePhysicalDescriptor([0.25; 15]),
                Tick::new(6),
            )
            .unwrap();
        assert_ne!(registry.save_state(observer_a).unwrap(), before_a);
        assert!(registry
            .record_by_key(observer_b, provenance(10).canonical_key())
            .is_some());

        registry.rollback_staged_transaction();
        assert_eq!(registry.save_state(observer_a).unwrap(), before_a);
        assert_eq!(registry.save_state(observer_b).unwrap(), before_b);
        assert!(registry
            .record_by_key(observer_b, provenance(10).canonical_key())
            .is_none());

        registry.begin_staged_transaction();
        registry
            .observe_batch(observer_b, [(provenance(11), descriptor(), Tick::new(7))])
            .unwrap();
        registry.commit_staged_transaction();
        assert!(registry
            .record_by_key(observer_b, provenance(11).canonical_key())
            .is_some());
    }
}
