use super::*;

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MemoryBucketKey {
    pub(super) organism_id_raw: u64,
    pub(super) profile_id_raw: u16,
    pub(super) profile_schema_version: u16,
    pub(super) sensory_abi_version_raw: u16,
    pub(super) query_version_raw: u16,
    pub(super) tracked_object_id_raw: u64,
    pub(super) family_raw: u16,
    pub(super) other_action_id_raw: u32,
    pub(super) target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TargetMemoryBucketKey {
    pub(super) organism_id_raw: u64,
    pub(super) profile_id_raw: u16,
    pub(super) profile_schema_version: u16,
    pub(super) sensory_abi_version_raw: u16,
    pub(super) query_version_raw: u16,
    pub(super) tracked_object_id_raw: u64,
    pub(super) target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MemoryRecordIdentity {
    pub(super) organism_id_raw: u64,
    pub(super) profile_id_raw: u16,
    pub(super) profile_schema_version: u16,
    pub(super) sensory_abi_version_raw: u16,
    pub(super) query_version_raw: u16,
    pub(super) tracked_object_id_raw: u64,
    pub(super) family_raw: u16,
    pub(super) other_action_id_raw: u32,
    pub(super) exact_target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct CandidateMemoryRecordV2 {
    pub(super) schema_version: u16,
    pub(super) memory_id: MemoryId,
    pub(super) organism_id_raw: u64,
    pub(super) source_sequence_id: ExperienceSequenceId,
    pub(super) first_tick: Tick,
    pub(super) last_tick: Tick,
    pub(super) profile_id_raw: u16,
    pub(super) profile_schema_version: u16,
    pub(super) sensory_abi_version_raw: u16,
    pub(super) query_version_raw: u16,
    pub(super) action_id_raw: u32,
    pub(super) action_kind_raw: u8,
    pub(super) family_raw: u16,
    pub(super) tracked_object_id_raw: u64,
    pub(super) query_features: Vec<f32>,
    pub(super) target_latent: [f32; MEMORY_LATENT_V1_COUNT],
    pub(super) family_value: [f32; MEMORY_VALUE_V1_COUNT],
    pub(super) confidence: f32,
    pub(super) salience_q16: u16,
    pub(super) observation_count: u32,
}

impl CandidateMemoryRecordV2 {
    pub(super) fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MEMORY_RECALL_SCHEMA_VERSION
            || self.organism_id_raw == 0
            || self.query_version_raw != crate::MemoryQueryVersion::StateActionTargetV2.raw()
            || self.query_features.len() != crate::MEMORY_QUERY_V2_FEATURE_COUNT
            || self
                .query_features
                .iter()
                .chain(self.target_latent.iter())
                .chain(self.family_value.iter())
                .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
            || !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
            || self.observation_count == 0
            || self.first_tick.raw() > self.last_tick.raw()
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        self.memory_id.validate()?;
        self.source_sequence_id.validate()?;
        ActionId(self.action_id_raw).validate()?;
        ActionKind::try_from_raw(self.action_kind_raw)?;
        let family = CandidateActionFamily::try_from_raw(
            u8::try_from(self.family_raw).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        )?;
        if !family.is_compatible_with(ActionKind::try_from_raw(self.action_kind_raw)?) {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }

    pub(super) fn identity(&self) -> MemoryRecordIdentity {
        MemoryRecordIdentity {
            organism_id_raw: self.organism_id_raw,
            profile_id_raw: self.profile_id_raw,
            profile_schema_version: self.profile_schema_version,
            sensory_abi_version_raw: self.sensory_abi_version_raw,
            query_version_raw: self.query_version_raw,
            tracked_object_id_raw: self.tracked_object_id_raw,
            family_raw: self.family_raw,
            other_action_id_raw: if self.family_raw == u16::from(CandidateActionFamily::Other.raw())
            {
                self.action_id_raw
            } else {
                0
            },
            exact_target_bins: target_bins(&self.query_features),
        }
    }

    pub(super) fn family_key(&self) -> MemoryBucketKey {
        let identity = self.identity();
        MemoryBucketKey {
            organism_id_raw: identity.organism_id_raw,
            profile_id_raw: identity.profile_id_raw,
            profile_schema_version: identity.profile_schema_version,
            sensory_abi_version_raw: identity.sensory_abi_version_raw,
            query_version_raw: identity.query_version_raw,
            tracked_object_id_raw: identity.tracked_object_id_raw,
            family_raw: identity.family_raw,
            other_action_id_raw: identity.other_action_id_raw,
            target_bins: identity.exact_target_bins,
        }
    }

    pub(super) fn target_key(&self) -> TargetMemoryBucketKey {
        let identity = self.identity();
        TargetMemoryBucketKey {
            organism_id_raw: identity.organism_id_raw,
            profile_id_raw: identity.profile_id_raw,
            profile_schema_version: identity.profile_schema_version,
            sensory_abi_version_raw: identity.sensory_abi_version_raw,
            query_version_raw: identity.query_version_raw,
            tracked_object_id_raw: identity.tracked_object_id_raw,
            target_bins: identity.exact_target_bins,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct CandidateMemoryStoreV2 {
    pub(super) generation: u64,
    pub(super) next_memory_id: u64,
    pub(super) merge_count: u64,
    pub(super) eviction_count: u64,
    pub(super) records: BTreeMap<u64, CandidateMemoryRecordV2>,
    pub(super) last_sequence_by_organism: BTreeMap<u64, u64>,
    #[serde(skip, default)]
    pub(super) family_index: BTreeMap<MemoryBucketKey, Vec<MemoryId>>,
    #[serde(skip, default)]
    pub(super) target_index: BTreeMap<TargetMemoryBucketKey, Vec<MemoryId>>,
}

impl Default for CandidateMemoryStoreV2 {
    fn default() -> Self {
        Self {
            generation: 0,
            next_memory_id: 1,
            merge_count: 0,
            eviction_count: 0,
            records: BTreeMap::new(),
            last_sequence_by_organism: BTreeMap::new(),
            family_index: BTreeMap::new(),
            target_index: BTreeMap::new(),
        }
    }
}

impl<'de> Deserialize<'de> for CandidateMemoryStoreV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            generation: u64,
            next_memory_id: u64,
            merge_count: u64,
            eviction_count: u64,
            records: BTreeMap<u64, CandidateMemoryRecordV2>,
            last_sequence_by_organism: BTreeMap<u64, u64>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let highest_id = wire.records.keys().next_back().copied().unwrap_or(0);
        if wire.next_memory_id == 0
            || wire.next_memory_id <= highest_id
            || (wire.generation == 0 && !wire.records.is_empty())
            || wire
                .records
                .iter()
                .any(|(raw_id, record)| *raw_id != record.memory_id.raw())
            || wire
                .last_sequence_by_organism
                .iter()
                .any(|(organism, sequence)| *organism == 0 || *sequence == 0)
        {
            return Err(D::Error::custom("invalid candidate memory store identity"));
        }
        for record in wire.records.values() {
            record.validate_contract().map_err(D::Error::custom)?;
            let last_sequence = wire
                .last_sequence_by_organism
                .get(&record.organism_id_raw)
                .copied()
                .ok_or_else(|| D::Error::custom("candidate memory sequence guard missing"))?;
            if record.source_sequence_id.raw() > last_sequence {
                return Err(D::Error::custom(
                    "candidate memory sequence guard predates record",
                ));
            }
        }
        let mut store = Self {
            generation: wire.generation,
            next_memory_id: wire.next_memory_id,
            merge_count: wire.merge_count,
            eviction_count: wire.eviction_count,
            records: wire.records,
            last_sequence_by_organism: wire.last_sequence_by_organism,
            family_index: BTreeMap::new(),
            target_index: BTreeMap::new(),
        };
        store.rebuild_indices();
        Ok(store)
    }
}

impl CandidateMemoryStoreV2 {
    pub(super) fn validate_for_capacity(
        &self,
        capacity: usize,
    ) -> Result<(), ScaffoldContractError> {
        let highest_id = self.records.keys().next_back().copied().unwrap_or(0);
        if capacity == 0
            || capacity > MEMORY_BANK_MAX_CAPACITY
            || self.records.len() > capacity
            || self.next_memory_id == 0
            || self.next_memory_id <= highest_id
            || (self.generation == 0 && !self.records.is_empty())
            || self
                .records
                .iter()
                .any(|(raw_id, record)| *raw_id != record.memory_id.raw())
            || self
                .last_sequence_by_organism
                .iter()
                .any(|(organism, sequence)| *organism == 0 || *sequence == 0)
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        for record in self.records.values() {
            record.validate_contract()?;
            let last_sequence = self
                .last_sequence_by_organism
                .get(&record.organism_id_raw)
                .copied()
                .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
            if record.source_sequence_id.raw() > last_sequence {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }

    pub(super) fn digest(&self, capacity: usize) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_for_capacity(capacity)?;
        let mut digest = CanonicalDigestBuilder::new(CANDIDATE_MEMORY_BANK_DOMAIN);
        digest.write_u16(MEMORY_RECALL_SCHEMA_VERSION);
        digest.write_u64(capacity as u64);
        digest.write_u64(self.generation);
        digest.write_u64(self.next_memory_id);
        digest.write_u64(self.merge_count);
        digest.write_u64(self.eviction_count);
        digest.write_sequence_len(self.records.len());
        for (raw_id, record) in &self.records {
            record.validate_contract()?;
            digest.write_u64(*raw_id);
            digest.write_u16(record.schema_version);
            digest.write_u64(record.memory_id.raw());
            digest.write_u64(record.organism_id_raw);
            digest.write_u64(record.source_sequence_id.raw());
            digest.write_u64(record.first_tick.raw());
            digest.write_u64(record.last_tick.raw());
            digest.write_u16(record.profile_id_raw);
            digest.write_u16(record.profile_schema_version);
            digest.write_u16(record.sensory_abi_version_raw);
            digest.write_u16(record.query_version_raw);
            digest.write_u32(record.action_id_raw);
            digest.write_u8(record.action_kind_raw);
            digest.write_u16(record.family_raw);
            digest.write_u64(record.tracked_object_id_raw);
            digest.write_sequence_len(record.query_features.len());
            for value in &record.query_features {
                digest.write_f32(*value)?;
            }
            digest.write_sequence_len(record.target_latent.len());
            for value in record.target_latent {
                digest.write_f32(value)?;
            }
            digest.write_sequence_len(record.family_value.len());
            for value in record.family_value {
                digest.write_f32(value)?;
            }
            digest.write_f32(record.confidence)?;
            digest.write_u16(record.salience_q16);
            digest.write_u32(record.observation_count);
        }
        digest.write_sequence_len(self.last_sequence_by_organism.len());
        for (organism, sequence) in &self.last_sequence_by_organism {
            digest.write_u64(*organism);
            digest.write_u64(*sequence);
        }
        Ok(digest.finish256())
    }

    pub(super) fn insert_record_into_indices(&mut self, memory_id: MemoryId) {
        let record = self
            .records
            .get(&memory_id.raw())
            .expect("validated candidate memory record exists");
        let family_key = record.family_key();
        let target_key = record.target_key();
        self.insert_family_index_id(family_key, memory_id);
        self.insert_target_index_id(target_key, memory_id);
    }

    fn insert_family_index_id(&mut self, family_key: MemoryBucketKey, memory_id: MemoryId) {
        let mut ids = self.family_index.remove(&family_key).unwrap_or_default();
        if !ids.contains(&memory_id) {
            ids.push(memory_id);
        }
        self.rank_index_ids(&mut ids);
        self.family_index.insert(family_key, ids);
    }

    fn insert_target_index_id(&mut self, target_key: TargetMemoryBucketKey, memory_id: MemoryId) {
        let mut ids = self.target_index.remove(&target_key).unwrap_or_default();
        if !ids.contains(&memory_id) {
            ids.push(memory_id);
        }
        self.rank_index_ids(&mut ids);
        self.target_index.insert(target_key, ids);
    }

    fn rank_index_ids(&self, ids: &mut [MemoryId]) {
        ids.sort_by(|left, right| {
            let left_record = &self.records[&left.raw()];
            let right_record = &self.records[&right.raw()];
            right_record
                .salience_q16
                .cmp(&left_record.salience_q16)
                .then_with(|| {
                    right_record
                        .last_tick
                        .raw()
                        .cmp(&left_record.last_tick.raw())
                })
                .then_with(|| {
                    right_record
                        .observation_count
                        .cmp(&left_record.observation_count)
                })
                .then_with(|| left.raw().cmp(&right.raw()))
        });
    }

    pub(super) fn remove_record_from_indices(&mut self, record: &CandidateMemoryRecordV2) {
        let family_key = record.family_key();
        if let Some(ids) = self.family_index.get_mut(&family_key) {
            ids.retain(|id| *id != record.memory_id);
            if ids.is_empty() {
                self.family_index.remove(&family_key);
            }
        }
        let target_key = record.target_key();
        if let Some(ids) = self.target_index.get_mut(&target_key) {
            ids.retain(|id| *id != record.memory_id);
            if ids.is_empty() {
                self.target_index.remove(&target_key);
            }
        }
    }

    pub(super) fn rebuild_indices(&mut self) {
        self.family_index.clear();
        self.target_index.clear();
        let ids = self
            .records
            .keys()
            .copied()
            .map(MemoryId)
            .collect::<Vec<_>>();
        for memory_id in ids {
            self.insert_record_into_indices(memory_id);
        }
    }
}

impl MemoryBucketKey {
    pub(super) fn receipt(&self) -> MemoryBucketReceiptKey {
        MemoryBucketReceiptKey {
            organism_id_raw: self.organism_id_raw,
            profile_id_raw: self.profile_id_raw,
            profile_schema_version: self.profile_schema_version,
            sensory_abi_version_raw: self.sensory_abi_version_raw,
            query_version_raw: self.query_version_raw,
            tracked_object_id_raw: self.tracked_object_id_raw,
            family_raw: self.family_raw,
            other_action_id_raw: self.other_action_id_raw,
            target_bins: self.target_bins,
        }
    }
}

impl TargetMemoryBucketKey {
    pub(super) fn receipt(&self) -> TargetMemoryBucketReceiptKey {
        TargetMemoryBucketReceiptKey {
            organism_id_raw: self.organism_id_raw,
            profile_id_raw: self.profile_id_raw,
            profile_schema_version: self.profile_schema_version,
            sensory_abi_version_raw: self.sensory_abi_version_raw,
            query_version_raw: self.query_version_raw,
            tracked_object_id_raw: self.tracked_object_id_raw,
            target_bins: self.target_bins,
        }
    }
}

pub(super) fn target_bins(features: &[f32]) -> [i8; CANDIDATE_FEATURE_COUNT] {
    let mut bins = [0_i8; CANDIDATE_FEATURE_COUNT];
    for (output, value) in bins.iter_mut().zip(features[MEMORY_TARGET_RANGE].iter()) {
        *output = (value.clamp(-1.0, 1.0) * 7.0).round() as i8;
    }
    bins
}

pub(super) fn keys_for_query(
    query: &CandidateMemoryQueryV2,
) -> (TargetMemoryBucketKey, MemoryBucketKey) {
    let bins = target_bins(query.features());
    let tracked_object_id_raw = query.tracked_object_id().map_or(0, |id| id.raw());
    let target = TargetMemoryBucketKey {
        organism_id_raw: query.organism_id().raw(),
        profile_id_raw: query.profile().profile_id.raw(),
        profile_schema_version: query.profile().profile_schema_version,
        sensory_abi_version_raw: query.profile().sensory_abi_version,
        query_version_raw: query.version().raw(),
        tracked_object_id_raw,
        target_bins: bins,
    };
    let family = MemoryBucketKey {
        organism_id_raw: target.organism_id_raw,
        profile_id_raw: target.profile_id_raw,
        profile_schema_version: target.profile_schema_version,
        sensory_abi_version_raw: target.sensory_abi_version_raw,
        query_version_raw: target.query_version_raw,
        tracked_object_id_raw,
        family_raw: u16::from(query.action_family().raw()),
        other_action_id_raw: if query.action_family() == CandidateActionFamily::Other {
            query.action_id().raw()
        } else {
            0
        },
        target_bins: bins,
    };
    (target, family)
}
