//! Separate factual execution recall. Never relabels scalar bundle consequences.
use super::*;
use crate::experience::ChannelPhysicalOutcome;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedActionMemoryV1 {
    pub query: CandidateMemoryQueryV2,
    pub source_sequence_id: ExperienceSequenceId,
    pub event_tick: Tick,
    pub bundle_digest: [u64; 4],
    pub receipt: ChannelPhysicalOutcome,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutedActionRecallV1 {
    pub candidate_index: u16,
    pub memory_id: MemoryId,
    pub evidence: ExecutedActionMemoryV1,
}
/// Read-only offboard result. This diagnostic work metric is not a brain debit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionEvidenceInspectionV1 {
    pub rows: Vec<Option<ExecutedActionRecallV1>>,
    pub diagnostic_work_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExecutionKey(u64, u16, u16, u16, u64, u8, u32, u16);
pub(super) fn key(q: &CandidateMemoryQueryV2) -> Option<ExecutionKey> {
    let p = q.profile();
    Some(ExecutionKey(
        q.organism_id().raw(),
        p.profile_id.raw(),
        p.profile_schema_version,
        p.sensory_abi_version,
        q.tracked_object_id()?.raw(),
        q.action_family().raw(),
        q.action_id().raw(),
        q.version().raw(),
    ))
}
/// Internal immutable pair. Bytes and validated decoded facts can only be
/// constructed together; serde preserves the original byte-array wire exactly.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CheckedExecutionFacts {
    bytes: Vec<u8>,
    facts: Vec<ExecutedActionMemoryV1>,
}
impl CheckedExecutionFacts {
    pub(super) fn from_bytes(bytes: Vec<u8>) -> Result<Self, ScaffoldContractError> {
        let facts = decode_bytes(&bytes)?;
        Ok(Self { bytes, facts })
    }
    fn from_facts(facts: Vec<ExecutedActionMemoryV1>) -> Result<Self, ScaffoldContractError> {
        validate_facts(&facts)?;
        let bytes =
            serde_json::to_vec(&facts).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        if bytes.len() > 65_536 {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(Self { bytes, facts })
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(super) fn facts(&self) -> &[ExecutedActionMemoryV1] {
        &self.facts
    }
}
impl Serialize for CheckedExecutionFacts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bytes.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for CheckedExecutionFacts {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Self::from_bytes(bytes).map_err(serde::de::Error::custom)
    }
}
fn decode_bytes(bytes: &[u8]) -> Result<Vec<ExecutedActionMemoryV1>, ScaffoldContractError> {
    if bytes.len() > 65_536 {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let facts: Vec<ExecutedActionMemoryV1> =
        serde_json::from_slice(bytes).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    validate_facts(&facts)?;
    Ok(facts)
}
fn validate_facts(facts: &[ExecutedActionMemoryV1]) -> Result<(), ScaffoldContractError> {
    if facts.is_empty() || facts.len() > crate::MAX_MOTOR_CHANNELS {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    for (i, fact) in facts.iter().enumerate() {
        fact.query.validate_contract()?;
        fact.source_sequence_id.validate()?;
        fact.receipt.validate_contract()?;
        let evidence = fact
            .receipt
            .execution_evidence
            .as_ref()
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        if fact.bundle_digest == [0; 4]
            || evidence.primitive != fact.query.action_id()
            || fact.event_tick.raw() < fact.query.tick().raw()
            || facts[..i]
                .iter()
                .any(|other| other.receipt.channel == fact.receipt.channel)
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
    }
    Ok(())
}
pub(super) fn cached(value: &Option<CheckedExecutionFacts>) -> &[ExecutedActionMemoryV1] {
    value.as_ref().map_or(&[], CheckedExecutionFacts::facts)
}
pub(super) fn from_patch(
    patch: &ExperiencePatch,
) -> Result<Option<CheckedExecutionFacts>, ScaffoldContractError> {
    let Some(bundle) = patch.selected_bundle() else {
        return Ok(None);
    };
    let Some(joint) = patch.outcome().joint.as_ref() else {
        return Ok(None);
    };
    let frame = patch.pre_action().perception();
    let mut facts = Vec::new();
    for receipt in &joint.channel_outcomes {
        let Some(evidence) = receipt.execution_evidence.as_ref() else {
            continue;
        };
        let command = bundle
            .channels
            .iter()
            .find(|c| c.channel == receipt.channel)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if command.primitive != evidence.primitive
            || command.target.and_then(|t| t.entity) != evidence.command_target
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut matches = frame.candidates().iter().filter(|candidate| {
            candidate.action_id == command.primitive
                && candidate.target.entity == evidence.command_target
                && candidate
                    .to_command(frame.organism_id(), command.confidence)
                    .ok()
                    .and_then(|action| {
                        crate::channel_command_for_action(receipt.channel, &action).ok()
                    })
                    .is_some_and(|expected| expected.target == command.target)
        });
        let candidate = matches
            .next()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if matches.next().is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let query = MemoryQueryEncoderV2::encode_from_frame(frame, candidate)?;
        facts.push(ExecutedActionMemoryV1 {
            query,
            source_sequence_id: patch.header().sequence_id,
            event_tick: patch.outcome().outcome_tick,
            bundle_digest: bundle.canonical_digest()?,
            receipt: *receipt,
        });
    }
    if facts.is_empty() {
        return Ok(None);
    }
    Ok(Some(CheckedExecutionFacts::from_facts(facts)?))
}
pub(super) fn write_digest(
    digest: &mut CanonicalDigestBuilder,
    bytes: &Option<Vec<u8>>,
) -> Result<(), ScaffoldContractError> {
    if let Some(bytes) = bytes {
        decode_bytes(bytes)?;
        digest.write_bytes(b"ALIFE-MEMORY-EXECUTED-ACTIONS-V1");
        digest.write_sequence_len(bytes.len());
        digest.write_bytes(bytes);
    }
    Ok(())
}
/// Internal cache is already checked; authenticate original bytes without reparsing.
pub(super) fn write_cached_digest(
    digest: &mut CanonicalDigestBuilder,
    value: &Option<CheckedExecutionFacts>,
) {
    if let Some(value) = value {
        digest.write_bytes(b"ALIFE-MEMORY-EXECUTED-ACTIONS-V1");
        digest.write_sequence_len(value.bytes().len());
        digest.write_bytes(value.bytes());
    }
}
pub(super) fn index_insert(
    index: &mut BTreeMap<ExecutionKey, Vec<(u64, MemoryId)>>,
    record: &CandidateMemoryRecordV2,
) {
    for fact in cached(&record.executed_actions_json) {
        if let Some(key) = key(&fact.query) {
            let ids = index.entry(key).or_default();
            let entry = (fact.source_sequence_id.raw(), record.memory_id);
            if !ids.iter().any(|(_, id)| *id == record.memory_id) {
                let position = ids
                    .binary_search_by_key(&entry.0, |(sequence, _)| *sequence)
                    .unwrap_or_else(|i| i);
                ids.insert(position, entry);
            }
        }
    }
}
pub(super) fn index_remove(
    index: &mut BTreeMap<ExecutionKey, Vec<(u64, MemoryId)>>,
    record: &CandidateMemoryRecordV2,
) {
    for fact in cached(&record.executed_actions_json) {
        if let Some(key) = key(&fact.query) {
            if let Some(ids) = index.get_mut(&key) {
                ids.retain(|(_, id)| *id != record.memory_id);
                if ids.is_empty() {
                    index.remove(&key);
                }
            }
        }
    }
}
pub(super) fn recall(
    store: &CandidateMemoryStoreV2,
    queries: &[CandidateMemoryQueryV2],
) -> Result<(Vec<Option<ExecutedActionRecallV1>>, u64), ScaffoldContractError> {
    if store.execution_index.is_empty() {
        return Ok((vec![None; queries.len()], 0));
    }
    let mut work = 0u64;
    let rows = queries
        .iter()
        .map(|query| {
            let Some(lookup) = key(query) else {
                return Ok(None);
            };
            work += 1; // Exact execution-index lookup.
            let mut best: Option<(MemoryId, &ExecutedActionMemoryV1)> = None;
            let mut ambiguous = false;
            for (_, id) in store
                .execution_index
                .get(&lookup)
                .into_iter()
                .flatten()
                .rev()
                .take(MEMORY_FAMILY_SEARCH_CAP)
            {
                let stored = &store.records[&id.raw()].executed_actions_json;
                // Checked immutable facts are decoded only at admission/cold load.
                // Explicit lookup charges record access and factual comparisons;
                // offboard callers report these units without a biological debit.
                work += 1;
                for fact in cached(stored) {
                    work += 1;
                    if key(&fact.query) != Some(lookup.clone())
                        || fact.event_tick.raw() > query.tick().raw()
                    {
                        continue;
                    }
                    if best.as_ref().is_none_or(|b| {
                        fact.source_sequence_id.raw() > b.1.source_sequence_id.raw()
                    }) {
                        best = Some((*id, fact));
                        ambiguous = false;
                    } else if let Some(previous) = &best {
                        if fact.source_sequence_id == previous.1.source_sequence_id
                            && (fact.receipt.physical != previous.1.receipt.physical
                                || fact.receipt.execution_evidence
                                    != previous.1.receipt.execution_evidence)
                        {
                            ambiguous = true;
                        }
                    }
                }
            }
            if ambiguous {
                return Ok(None);
            }
            Ok(best.map(|(memory_id, fact)| {
                work += 1; // Final selected factual projection; no per-record clone.
                ExecutedActionRecallV1 {
                    candidate_index: query.candidate_index(),
                    memory_id,
                    evidence: fact.clone(),
                }
            }))
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    Ok((rows, work))
}

pub(super) fn validate_projection(
    rows: &[Option<ExecutedActionRecallV1>],
    queries: &[&CandidateMemoryQueryV2],
) -> Result<(), ScaffoldContractError> {
    if rows.len() != queries.len() {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    for (row, query) in rows.iter().zip(queries) {
        if let Some(row) = row {
            if row.candidate_index != query.candidate_index()
                || key(&row.evidence.query) != key(query)
                || key(query).is_none()
                || row.evidence.event_tick.raw() > query.tick().raw()
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
    }
    Ok(())
}
