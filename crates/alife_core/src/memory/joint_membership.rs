//! Opt-in joint-observed association, never isolated action physiology.
use super::*;
use std::collections::BTreeSet;

pub(super) const RECORD_SCHEMA: u16 = 3;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    schema_version: u16,
    sealed_causal_digest: [u64; 4],
    members: executed_action::CheckedExecutionFacts,
    shared_family_value: [f32; MEMORY_VALUE_V1_COUNT],
    event_confidence: f32,
}
/// Immutable bytes/cache pair. New event values are captured before episode merge.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CheckedJointEvent {
    bytes: Vec<u8>,
    event: Event,
}
impl CheckedJointEvent {
    pub(super) fn from_bytes(bytes: Vec<u8>) -> Result<Self, ScaffoldContractError> {
        if bytes.len() > 131_072 {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let event: Event = serde_json::from_slice(&bytes)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        if event.schema_version != 1
            || event.sealed_causal_digest == [0; 4]
            || !event.event_confidence.is_finite()
            || !(0.0..=1.0).contains(&event.event_confidence)
            || event
                .shared_family_value
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let first = &event.members.facts()[0];
        if event.members.facts().iter().any(|f| {
            f.source_sequence_id != first.source_sequence_id
                || f.event_tick != first.event_tick
                || f.bundle_digest != first.bundle_digest
                || f.query.organism_id() != first.query.organism_id()
                || f.query.profile() != first.query.profile()
                || f.query.tick() != first.query.tick()
        }) {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(Self { bytes, event })
    }
    pub(super) fn capture(
        record: &CandidateMemoryRecordV2,
        patch: &ExperiencePatch,
    ) -> Result<Option<Self>, ScaffoldContractError> {
        let Some(members) = record.executed_actions_json.clone() else {
            return Ok(None);
        };
        let event = Event {
            schema_version: 1,
            sealed_causal_digest: patch.causal_digest()?,
            members,
            shared_family_value: record.family_value,
            event_confidence: record.confidence,
        };
        Self::from_bytes(
            serde_json::to_vec(&event).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        )
        .map(Some)
    }
    pub(super) fn validate_portable(
        &self,
        record: &PortableMemoryRecordV2,
    ) -> Result<(), ScaffoldContractError> {
        for f in self.event.members.facts() {
            let q = &f.query;
            if q.organism_id().raw() != record.organism_id_raw
                || q.profile().profile_id.raw() != record.profile_id_raw
                || q.profile().profile_schema_version != record.profile_schema_version
                || q.profile().sensory_abi_version != record.sensory_abi_version_raw
                || q.version().raw() != record.query_version_raw
                || q.tick().raw() < record.first_tick_raw
                || q.tick().raw() > record.last_tick_raw
                || f.source_sequence_id.raw() > record.sealed_sequence_id_raw
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }
    pub(super) fn admission_work(&self) -> u64 {
        // Two passes (encoding + checked decoding), plus member validation and
        // member validation. Index maintenance is counted separately. This is a declared logical work metric.
        2 * self.bytes.len().div_ceil(32) as u64 + self.event.members.facts().len() as u64
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(super) fn sequence(&self) -> u64 {
        self.event.members.facts()[0].source_sequence_id.raw()
    }
    pub(super) fn validate_owner(
        &self,
        record: &CandidateMemoryRecordV2,
    ) -> Result<(), ScaffoldContractError> {
        for f in self.event.members.facts() {
            let q = &f.query;
            if q.organism_id().raw() != record.organism_id_raw
                || q.profile().profile_id.raw() != record.profile_id_raw
                || q.profile().profile_schema_version != record.profile_schema_version
                || q.profile().sensory_abi_version != record.sensory_abi_version_raw
                || q.version().raw() != record.query_version_raw
                || q.tick() < record.first_tick
                || q.tick() > record.last_tick
                || f.source_sequence_id.raw() > record.source_sequence_id.raw()
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }
}
impl Serialize for CheckedJointEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bytes.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for CheckedJointEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::from_bytes(Vec::<u8>::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
pub(super) fn digest(d: &mut CanonicalDigestBuilder, event: &Option<CheckedJointEvent>) {
    if let Some(e) = event {
        d.write_bytes(b"ALIFE-JOINT-OBSERVED-EVENT-V1");
        d.write_sequence_len(e.bytes.len());
        d.write_bytes(&e.bytes);
    }
}
/// Sources for an alternative family estimate. Values remain whole-joint outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointMembershipSourceV1 {
    pub candidate_index: u16,
    pub memory_id: MemoryId,
    pub source_sequence_id: ExperienceSequenceId,
    pub bundle_digest: [u64; 4],
}
fn matches(a: &CandidateMemoryQueryV2, b: &CandidateMemoryQueryV2) -> bool {
    a.organism_id() == b.organism_id()
        && a.profile() == b.profile()
        && a.version() == b.version()
        && a.tracked_object_id().is_some()
        && a.tracked_object_id() == b.tracked_object_id()
        && a.action_family() == b.action_family()
        && a.action_id() == b.action_id()
}
impl MemoryBank {
    /// Bound prospective event admission from actual executed facts, without
    /// manufacturing physiological values or a provisional sealed patch.
    /// The returned tariff includes the planning pass and a reserved admission
    /// ceiling. It is deliberately not a claim that all reserved work executed.
    pub fn reserve_joint_membership_admission_work(
        &self,
        frame: &PerceptionFrame,
        bundle: &crate::MotorCommandBundle,
        joint: &crate::JointPhysicalOutcome,
        sequence: ExperienceSequenceId,
        event_tick: Tick,
    ) -> Result<(u64, u64), ScaffoldContractError> {
        let n = self.candidate_store.records.len() as u64;
        let comparisons = u64::from((n + 1).ilog2()) + 2;
        let Some(facts) =
            executed_action::from_execution(frame, bundle, joint, sequence, event_tick)?
        else {
            // An unknown new event may still merge into a record retaining an
            // earlier event, so reserve removal/reinsertion of that old index.
            return Ok((
                1 + joint.channel_outcomes.len() as u64,
                crate::motor::MAX_MOTOR_CHANNELS as u64 * (2 * n + comparisons + 5),
            ));
        };
        let members =
            serde_json::to_vec(&facts).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        // Exact serialized member bytes, plus a ceiling for fixed JSON keys,
        // four u64 digest limbs and five finite f32 scalars. 512 exceeds their
        // maximum JSON text length and does not depend on the eventual values.
        let event_bytes = members
            .len()
            .checked_add(512)
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        let count = facts.facts().len() as u64;
        // At most one old record removed and one new/merged record inserted.
        // An old event can have at most MAX_MOTOR_CHANNELS members.
        let index_bound =
            crate::motor::MAX_MOTOR_CHANNELS as u64 * (n + 2) + count * (n + comparisons + 3);
        let admission = 2 * event_bytes.div_ceil(32) as u64 + count + index_bound;
        let planning = 2 * members.len().div_ceil(32) as u64
            + count
            + 1
            + frame.candidates().len() as u64 * joint.channel_outcomes.len() as u64;
        Ok((planning, admission))
    }

    /// Capture one exact joint consequence with its authenticated executed members.
    /// Does not alter neural eligibility/biology or manufacture member-local outcomes.
    pub fn observe_sealed_patch_with_joint_membership(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryUpdateReceipt, ScaffoldContractError> {
        self.observe_sealed_patch_impl(patch, false, true)
    }
    /// Explicit alternative family estimator. Target recall is unchanged. Events
    /// are not added to ordinary family estimates: matching event estimates replace
    /// that candidate's scalar family estimate; absent evidence retains legacy recall.
    pub fn recall_frame_with_joint_membership(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<PreparedMemoryRecall, ScaffoldContractError> {
        self.apply_joint_membership(self.recall_frame(draft)?, draft)
    }

    /// Cost-matched research comparison: ordinary recall is computed once, then
    /// the same membership search executes whether its family values are applied
    /// or withheld. Returned sources describe the query, not necessarily the
    /// context used. The context receipt lists only applied sources.
    pub fn recall_frame_joint_comparison(
        &self,
        draft: &PerceptionFrameDraft,
        apply: bool,
    ) -> Result<(PreparedMemoryRecall, Vec<JointMembershipSourceV1>), ScaffoldContractError> {
        self.recall_frame_joint_comparison_impl(draft, apply, false)
    }

    /// Contact-only application candidate. Search and declared comparison work
    /// remain identical to the all-family comparison; no membership is removed.
    pub fn recall_frame_joint_contact_comparison2266(
        &self,
        draft: &PerceptionFrameDraft,
        apply: bool,
    ) -> Result<(PreparedMemoryRecall, Vec<JointMembershipSourceV1>), ScaffoldContractError> {
        self.recall_frame_joint_comparison_impl(draft, apply, true)
    }

    fn recall_frame_joint_comparison_impl(
        &self,
        draft: &PerceptionFrameDraft,
        apply: bool,
        contact_only: bool,
    ) -> Result<(PreparedMemoryRecall, Vec<JointMembershipSourceV1>), ScaffoldContractError> {
        let ordinary = self.recall_frame(draft)?;
        let families: Vec<_> = ordinary
            .context
            .candidates
            .iter()
            .map(|c| {
                (
                    c.family_value,
                    c.family_confidence,
                    c.family_source_count,
                    c.best_family_source,
                )
            })
            .collect();
        let mut p = self.apply_joint_membership(ordinary, draft)?;
        let queried = p.receipt.joint_membership_sources.clone();
        for ((c, q), old) in p
            .context
            .candidates
            .iter_mut()
            .zip(&p.candidate_queries)
            .zip(families)
        {
            let chosen = if apply
                && (!contact_only || q.action_family() == CandidateActionFamily::Contact)
            {
                (
                    c.family_value,
                    c.family_confidence,
                    c.family_source_count,
                    c.best_family_source,
                )
            } else {
                old
            };
            c.family_value = chosen.0;
            c.family_confidence = chosen.1;
            c.family_source_count = chosen.2;
            c.best_family_source = chosen.3;
        }
        // Equal declared comparison tariff: save and assign seven family scalar
        // fields per candidate, plus copy each queried source receipt and re-encode/hash the context. This is a
        // logical work metric, not measured machine instructions or wall time.
        let comparison_work = 14 * p.context.candidates.len() as u64
            + queried.len() as u64
            + 2 * p.encoded_context.values().len() as u64;
        p.receipt.execution_evidence_work = p
            .receipt
            .execution_evidence_work
            .checked_add(comparison_work)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        if contact_only {
            p.receipt.joint_membership_sources.retain(|source| {
                apply
                    && p.candidate_queries.iter().any(|q| {
                        q.candidate_index() == source.candidate_index
                            && q.action_family() == CandidateActionFamily::Contact
                    })
            });
        } else if !apply {
            p.receipt.joint_membership_sources.clear();
        }
        p.encoded_context = p.context.to_perception_context_block()?;
        p.receipt.context_digest = p.encoded_context.canonical_digest();
        p.validate_for_draft(draft)?;
        Ok((p, queried))
    }

    fn apply_joint_membership(
        &self,
        mut p: PreparedMemoryRecall,
        draft: &PerceptionFrameDraft,
    ) -> Result<PreparedMemoryRecall, ScaffoldContractError> {
        p.receipt.schema_version = RECORD_SCHEMA;
        let mut work = 0u64;
        for (i, q) in p.candidate_queries.iter().enumerate() {
            let mut seen = BTreeSet::new();
            let mut selected = Vec::new();
            let Some(key) = executed_action::key(q) else {
                continue;
            };
            if self.candidate_store.joint_membership_index.is_empty() {
                continue;
            }
            work += 1; // Exact member namespace index lookup.
                       // Latest matching event sequences, not record IDs (merged IDs stay old).
            for (_, id) in self
                .candidate_store
                .joint_membership_index
                .get(&key)
                .into_iter()
                .flatten()
                .rev()
                .take(MEMORY_FAMILY_SEARCH_CAP)
            {
                work += 1;
                let record = &self.candidate_store.records[&id.raw()];
                let Some(e) = &record.joint_event else {
                    continue;
                };
                let first = &e.event.members.facts()[0];
                if first.event_tick > q.tick() {
                    continue;
                }
                for f in e.event.members.facts() {
                    work += 1;
                    if !matches(&f.query, q) {
                        continue;
                    }
                    let similarity = family_similarity(q.features(), f.query.features());
                    if similarity < MEMORY_MIN_SIMILARITY {
                        continue;
                    }
                    let age = q.tick().raw().saturating_sub(first.event_tick.raw()) as f32;
                    let retention = (1.0 / (1.0 + age / 16384.0)).max(0.05);
                    // Confidence is ultimately a f32. Compute its unnormalized
                    // support in f64 so intermediate weighting cannot erase a
                    // usable subnormal. This is representability, not a biological
                    // confidence floor. Unusable events must not occupy top-K.
                    let support = f64::from(e.event.event_confidence)
                        * f64::from(similarity)
                        * f64::from(retention);
                    if support as f32 <= 0.0 {
                        continue;
                    }
                    let key = (f.query.organism_id().raw(), f.source_sequence_id.raw());
                    if seen.insert(key) {
                        selected.push((record.memory_id, e, similarity, support));
                    }
                }
            }
            selected.sort_by(|a, b| {
                work += 1; // Actual comparison in deterministic ranking.
                b.2.total_cmp(&a.2)
                    .then_with(|| b.1.sequence().cmp(&a.1.sequence()))
                    .then_with(|| a.0.raw().cmp(&b.0.raw()))
            });
            selected.truncate(MEMORY_RECALL_TOP_K);
            if selected.is_empty() {
                continue;
            }
            let total: f32 = selected.iter().map(|x| x.2).sum();
            let mut values = [0.0; MEMORY_VALUE_V1_COUNT];
            // Algebraically the old normalized confidence is the mean of
            // confidence * similarity * retention. Delay rounding until after
            // that mean; dividing each f32 subnormal by K can round it to zero.
            let confidence =
                (selected.iter().map(|x| x.3).sum::<f64>() / selected.len() as f64) as f32;
            if confidence <= 0.0 {
                continue;
            }
            let family_confidence = Confidence::new(confidence.clamp(0.0, 1.0))?;
            for (id, e, score, _) in &selected {
                let f = &e.event.members.facts()[0];
                for (v, source) in values.iter_mut().zip(e.event.shared_family_value) {
                    *v += source * (*score / total);
                }
                p.receipt
                    .joint_membership_sources
                    .push(JointMembershipSourceV1 {
                        candidate_index: q.candidate_index(),
                        memory_id: *id,
                        source_sequence_id: f.source_sequence_id,
                        bundle_digest: f.bundle_digest,
                    });
                work += 1;
            }
            let c = &mut p.context.candidates[i];
            c.family_value = values.map(|v| v.clamp(-1.0, 1.0));
            c.family_confidence = family_confidence;
            c.family_source_count = selected.len() as u16;
            c.best_family_source = Some(selected[0].0);
        }
        p.receipt.execution_evidence_work = p
            .receipt
            .execution_evidence_work
            .checked_add(work)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        p.encoded_context = p.context.to_perception_context_block()?;
        p.receipt.context_digest = p.encoded_context.canonical_digest();
        p.validate_for_draft(draft)?;
        Ok(p)
    }
    /// Contact-query-only comparison. Ordinary recall executes once; only Contact
    /// candidates enter membership lookup. Capture/admission are unchanged.
    /// Applied and withheld routes execute equal logical work on an identical bank.
    pub fn recall_frame_joint_contact_query_comparison2273(
        &self,
        draft: &PerceptionFrameDraft,
        apply: bool,
    ) -> Result<(PreparedMemoryRecall, Vec<JointMembershipSourceV1>), ScaffoldContractError> {
        let mut p = self.recall_frame(draft)?;
        let mut queried = Vec::new();
        p.receipt.schema_version = RECORD_SCHEMA;
        let mut work = 0u64;
        for (i, q) in p.candidate_queries.iter().enumerate() {
            if q.action_family() != CandidateActionFamily::Contact {
                continue;
            }
            let mut seen = BTreeSet::new();
            let mut selected = Vec::new();
            let Some(key) = executed_action::key(q) else {
                continue;
            };
            if self.candidate_store.joint_membership_index.is_empty() {
                continue;
            }
            work += 1; // Exact member namespace index lookup.
                       // Latest matching event sequences, not record IDs (merged IDs stay old).
            for (_, id) in self
                .candidate_store
                .joint_membership_index
                .get(&key)
                .into_iter()
                .flatten()
                .rev()
                .take(MEMORY_FAMILY_SEARCH_CAP)
            {
                work += 1;
                let record = &self.candidate_store.records[&id.raw()];
                let Some(e) = &record.joint_event else {
                    continue;
                };
                let first = &e.event.members.facts()[0];
                if first.event_tick > q.tick() {
                    continue;
                }
                for f in e.event.members.facts() {
                    work += 1;
                    if !matches(&f.query, q) {
                        continue;
                    }
                    let similarity = family_similarity(q.features(), f.query.features());
                    if similarity < MEMORY_MIN_SIMILARITY {
                        continue;
                    }
                    let age = q.tick().raw().saturating_sub(first.event_tick.raw()) as f32;
                    let retention = (1.0 / (1.0 + age / 16384.0)).max(0.05);
                    // Confidence is ultimately a f32. Compute its unnormalized
                    // support in f64 so intermediate weighting cannot erase a
                    // usable subnormal. This is representability, not a biological
                    // confidence floor. Unusable events must not occupy top-K.
                    let support = f64::from(e.event.event_confidence)
                        * f64::from(similarity)
                        * f64::from(retention);
                    if support as f32 <= 0.0 {
                        continue;
                    }
                    let key = (f.query.organism_id().raw(), f.source_sequence_id.raw());
                    if seen.insert(key) {
                        selected.push((record.memory_id, e, similarity, support));
                    }
                }
            }
            selected.sort_by(|a, b| {
                work += 1; // Actual comparison in deterministic ranking.
                b.2.total_cmp(&a.2)
                    .then_with(|| b.1.sequence().cmp(&a.1.sequence()))
                    .then_with(|| a.0.raw().cmp(&b.0.raw()))
            });
            selected.truncate(MEMORY_RECALL_TOP_K);
            if selected.is_empty() {
                continue;
            }
            let total: f32 = selected.iter().map(|x| x.2).sum();
            let mut values = [0.0; MEMORY_VALUE_V1_COUNT];
            // Algebraically the old normalized confidence is the mean of
            // confidence * similarity * retention. Delay rounding until after
            // that mean; dividing each f32 subnormal by K can round it to zero.
            let confidence =
                (selected.iter().map(|x| x.3).sum::<f64>() / selected.len() as f64) as f32;
            if confidence <= 0.0 {
                continue;
            }
            let family_confidence = Confidence::new(confidence.clamp(0.0, 1.0))?;
            for (id, e, score, _) in &selected {
                let f = &e.event.members.facts()[0];
                for (v, source) in values.iter_mut().zip(e.event.shared_family_value) {
                    *v += source * (*score / total);
                }
                queried.push(JointMembershipSourceV1 {
                    candidate_index: q.candidate_index(),
                    memory_id: *id,
                    source_sequence_id: f.source_sequence_id,
                    bundle_digest: f.bundle_digest,
                });
                work += 1;
            }
            let c = &mut p.context.candidates[i];
            // Save/select/assign seven scalar fields for both routes. No
            // non-Contact save/restore and no second ordinary lookup.
            let old = (
                c.family_value,
                c.family_confidence,
                c.family_source_count,
                c.best_family_source,
            );
            c.family_value = values.map(|v| v.clamp(-1.0, 1.0));
            c.family_confidence = family_confidence;
            c.family_source_count = selected.len() as u16;
            c.best_family_source = Some(selected[0].0);
            let chosen = if apply {
                (
                    c.family_value,
                    c.family_confidence,
                    c.family_source_count,
                    c.best_family_source,
                )
            } else {
                old
            };
            c.family_value = chosen.0;
            c.family_confidence = chosen.1;
            c.family_source_count = chosen.2;
            c.best_family_source = chosen.3;
            work += 21; // Save seven, construct seven, select/assign seven.
        }
        // Both routes copy query provenance and encode/hash once only if a
        // usable Contact estimate exists. A frame without Contact queries does
        // no membership lookup, comparison, or redundant encoding work.
        p.receipt.joint_membership_sources = queried.clone();
        work += queried.len() as u64;
        if !apply {
            p.receipt.joint_membership_sources.clear();
        }
        if !queried.is_empty() {
            p.encoded_context = p.context.to_perception_context_block()?;
            p.receipt.context_digest = p.encoded_context.canonical_digest();
            work += 2 * p.encoded_context.values().len() as u64;
        }
        p.receipt.execution_evidence_work = p
            .receipt
            .execution_evidence_work
            .checked_add(work)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        if queried.is_empty() {
            // Ordinary recall already validated the unchanged context and draft.
            // Only the receipt schema/work changed; avoid a redundant full hash.
            p.receipt.validate_contract()?;
        } else {
            p.validate_for_draft(draft)?;
        }
        Ok((p, queried))
    }
}

pub(super) fn index_insert(
    index: &mut std::collections::BTreeMap<executed_action::ExecutionKey, Vec<(u64, MemoryId)>>,
    record: &CandidateMemoryRecordV2,
) -> u64 {
    let mut work = 0;
    if let Some(event) = &record.joint_event {
        for fact in event.event.members.facts() {
            work += 1;
            if let Some(key) = executed_action::key(&fact.query) {
                work += 1;
                let entries = index.entry(key).or_default();
                let entry = (event.sequence(), record.memory_id);
                let location = entries.binary_search_by(|e| {
                    work += 1;
                    (e.0, e.1.raw()).cmp(&(entry.0, entry.1.raw()))
                });
                if let Err(position) = location {
                    work += (entries.len() - position + 1) as u64;
                    entries.insert(position, entry);
                }
            }
        }
    }
    work
}
pub(super) fn index_remove(
    index: &mut std::collections::BTreeMap<executed_action::ExecutionKey, Vec<(u64, MemoryId)>>,
    record: &CandidateMemoryRecordV2,
) -> u64 {
    let mut work = 0;
    if let Some(event) = &record.joint_event {
        for fact in event.event.members.facts() {
            work += 1;
            if let Some(key) = executed_action::key(&fact.query) {
                work += 1;
                if let Some(entries) = index.get_mut(&key) {
                    entries.retain(|e| {
                        work += 1;
                        e.1 != record.memory_id
                    });
                    if entries.is_empty() {
                        index.remove(&key);
                    }
                }
            }
        }
    }
    work
}
