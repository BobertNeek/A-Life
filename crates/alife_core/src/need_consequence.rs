//! Opt-in, exact-context evidence of measured biological interval consequences.
//!
//! This is deliberately separate from MemoryBank, its V2 formats, and all default
//! readers. Only a neural representative action with an observed tracked object
//! and entity can be admitted. Targetless self-actions (including Rest/Idle),
//! unobserved targets, and heuristic evidence remain Unknown even if measured;
//! this narrow target-association prototype does not deny their biological effects.
//! There is no migration from aggregate valence and no action selection.
//! One retained sealed source produces one association, including for a joint
//! bundle: its before/after physiology is NOT isolated causal credit for a motor
//! channel. All eleven normalized drive lanes are raw `after - before` values;
//! ATP availability, curiosity, reproductive drive and extension lanes are not
//! silently relabeled as deficits.
//!
//! The prototype retains bounded source JSON, at higher storage cost than a
//! compressed memory. Restore revalidates those sources and rederives every key
//! and delta. JSON is the only persistence entrypoint: input bytes are bounded
//! before parsing. These private acquired observations are not fixture assets.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::{
    ActionCandidate, ActionId, ActionKind, CandidateActionFamily, CandidateObservationRef,
    CanonicalDigestBuilder, Confidence, EvidenceKind, ExperiencePatch, ExperienceSequenceId,
    OrganismId, PerceptionFrame, ScaffoldContractError, SensorProfileIdentity, Tick,
    TrackedObjectId, Validate, WorldEntityId,
};

pub const NEED_CONSEQUENCE_SCHEMA_VERSION: u16 = 1;
pub const MAX_NEED_CONSEQUENCE_RECORDS: usize = 32;
pub const MAX_NEED_CONSEQUENCE_SOURCE_BYTES: usize = 128 * 1024;
pub const MAX_NEED_CONSEQUENCE_STORE_BYTES: usize = 1024 * 1024;
pub const MAX_NEED_CONSEQUENCE_QUERY_SCAN: usize = 32;
pub const MAX_NEED_CONSEQUENCE_QUERY_RESULTS: usize = 8;
pub const NEED_CONSEQUENCE_DRIVE_COUNT: usize = 11;

#[derive(Debug, thiserror::Error)]
pub enum NeedConsequenceErrorV1 {
    #[error(transparent)]
    Contract(#[from] ScaffoldContractError),
    #[error("need-consequence capacity or query budget is out of bounds")]
    InvalidLimit,
    #[error("need-consequence encoded byte limit exceeded")]
    ByteLimit,
    #[error("need-consequence source belongs to another organism")]
    ForeignOrganism,
    #[error("need-consequence source sequence was already observed or is older")]
    ReplayedSource,
    #[error("need-consequence source interval moves backward")]
    BackwardInterval,
    #[error("malformed or unsupported need-consequence store/source")]
    MalformedSource,
}

/// Exact association key, constructed only from a validated observed frame.
/// No fields are public and there is no detached/deserializable key constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedConsequenceQueryV1 {
    organism_id: OrganismId,
    profile: SensorProfileIdentity,
    action_id: ActionId,
    action_kind: ActionKind,
    family: CandidateActionFamily,
    tracked_target: Option<TrackedObjectId>,
    entity: Option<WorldEntityId>,
    context_digest: [u64; 4],
}

impl NeedConsequenceQueryV1 {
    pub fn from_frame(
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
    ) -> Result<Self, NeedConsequenceErrorV1> {
        // Bound the observation projection before deeper legacy frame validation.
        let context = exact_context_bytes(frame)?;
        frame.validate_contract()?;
        if frame
            .candidates()
            .get(usize::from(candidate.candidate_index))
            != Some(candidate)
        {
            return Err(NeedConsequenceErrorV1::MalformedSource);
        }
        let tracked_target = match candidate.observation {
            CandidateObservationRef::ObjectSlot(index) => Some(
                frame
                    .grounded_object_slots()
                    .get(usize::from(index))
                    .ok_or(NeedConsequenceErrorV1::MalformedSource)?
                    .tracked_object_id,
            ),
            CandidateObservationRef::None => None,
        };
        // Hash a dedicated projection of the validated original observation.
        // Never create/revalidate a fictitious tick-zero biological frame.
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-NEED-CONSEQUENCE-CONTEXT-V1");
        digest.write_bytes(&context);
        Ok(Self {
            organism_id: frame.organism_id(),
            profile: frame.profile_provenance().identity(),
            action_id: candidate.action_id,
            action_kind: candidate.kind,
            family: candidate.family,
            tracked_target,
            entity: candidate.target.entity,
            context_digest: digest.finish256(),
        })
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }
    pub const fn profile(&self) -> SensorProfileIdentity {
        self.profile
    }
    pub const fn action_family(&self) -> CandidateActionFamily {
        self.family
    }
    pub const fn tracked_target(&self) -> Option<TrackedObjectId> {
        self.tracked_target
    }
    pub const fn target_entity(&self) -> Option<WorldEntityId> {
        self.entity
    }
    pub const fn context_digest(&self) -> [u64; 4] {
        self.context_digest
    }
}

/// The only omitted clocks are frame.tick, sensory.tick, homeostasis.tick and
/// profile.source_tick. Their original values and original frame digest remain
/// in the retained source. All observed language, social, semantic, retrieval,
/// sensory and candidate fields are preserved, including any nested temporal
/// identity. No timestamp/digest inside observed payloads is rewritten.
fn exact_context_bytes(frame: &PerceptionFrame) -> Result<Vec<u8>, NeedConsequenceErrorV1> {
    #[derive(Serialize)]
    struct SensoryProjection<'a> {
        abi_version: crate::SensoryAbiVersion,
        organism_id: OrganismId,
        observer_position: crate::Vec3f,
        channels: &'a crate::SensoryChannels,
        context_streams: &'a crate::ContextStreams,
        social_context: &'a crate::SocialContextSnapshot,
        language_context: &'a crate::LanguageContextSnapshot,
        semantic_context: &'a Option<crate::SemanticContextRef>,
        gaussian_context: &'a Option<crate::GaussianContextRef>,
    }
    #[derive(Serialize)]
    struct ContextProjection<'a> {
        profile: SensorProfileIdentity,
        body: crate::BodySnapshot,
        drives: &'a crate::DriveSnapshot,
        hormones: &'a crate::EndocrineSnapshot,
        sensory: SensoryProjection<'a>,
        candidates: &'a [ActionCandidate],
        grounded_objects: &'a [crate::GroundedObjectSlotV1],
        retrieval_context: &'a crate::PerceptionContextBlock,
    }
    let sensory = frame.sensory();
    encode_bounded(
        &ContextProjection {
            profile: frame.profile_provenance().identity(),
            body: frame.body(),
            drives: &frame.homeostasis().drives,
            hormones: &frame.homeostasis().hormones,
            sensory: SensoryProjection {
                abi_version: sensory.abi_version,
                organism_id: sensory.organism_id,
                observer_position: sensory.observer_position,
                channels: &sensory.channels,
                context_streams: &sensory.context_streams,
                social_context: &sensory.social_context,
                language_context: &sensory.language_context,
                semantic_context: &sensory.semantic_context,
                gaussian_context: &sensory.gaussian_context,
            },
            candidates: frame.candidates(),
            grounded_objects: frame.grounded_object_slots(),
            retrieval_context: frame.context(),
        },
        MAX_NEED_CONSEQUENCE_SOURCE_BYTES,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedConsequenceAttributionV1 {
    /// The entire observed interval, whether it contained one command or many.
    JointIntervalAssociation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeedConsequenceObservationV1 {
    pub source_sequence_id: ExperienceSequenceId,
    pub before_tick: Tick,
    pub after_tick: Tick,
    /// Hunger, fatigue, fear, pain, loneliness, curiosity, brain ATP,
    /// temperature stress, reproductive drive, extension 0, extension 1.
    /// Units: signed change in the normalized biological drive, not reward.
    pub drive_deltas: [f32; NEED_CONSEQUENCE_DRIVE_COUNT],
    pub confidence: Confidence,
    pub attribution: NeedConsequenceAttributionV1,
    pub joint_command_count: usize,
    pub source_frame_digest: [u64; 4],
    pub source_bundle_digest: Option<[u64; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedConsequenceAdmissionV1 {
    Measured {
        pruned_records: usize,
    },
    /// The sequence is consumed even when measurement/observed target is absent.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NeedConsequenceEvidenceV1 {
    Unknown,
    /// Individual observations, newest first. No pooled or duplicated credit.
    Measured(Vec<NeedConsequenceObservationV1>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeedConsequenceRecallV1 {
    pub evidence: NeedConsequenceEvidenceV1,
    pub inspected_records: usize,
    /// True when the caller's scan or result budget left older sources unseen.
    pub budget_exhausted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWatermark {
    sequence: ExperienceSequenceId,
    before_tick: Tick,
    after_tick: Tick,
}

#[derive(Debug, Clone, PartialEq)]
struct Record {
    source_json: String,
    query: NeedConsequenceQueryV1,
    observation: NeedConsequenceObservationV1,
}

/// A separately instantiated, organism-owned optional store. No Default impl,
/// no runtime attachment, no legacy record import, and no shared population pool.
#[derive(Debug, Clone, PartialEq)]
pub struct NeedConsequenceStoreV1 {
    organism_id: OrganismId,
    capacity: usize,
    watermark: Option<SourceWatermark>,
    records: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreWire {
    schema_version: u16,
    organism_id: OrganismId,
    capacity: usize,
    watermark: Option<SourceWatermark>,
    sources: Vec<String>,
    integrity_digest: [u64; 4],
}

impl NeedConsequenceStoreV1 {
    pub fn new(organism_id: OrganismId, capacity: usize) -> Result<Self, NeedConsequenceErrorV1> {
        organism_id.validate()?;
        if capacity == 0 || capacity > MAX_NEED_CONSEQUENCE_RECORDS {
            return Err(NeedConsequenceErrorV1::InvalidLimit);
        }
        Ok(Self {
            organism_id,
            capacity,
            watermark: None,
            records: Vec::new(),
        })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }
    pub fn last_source_sequence(&self) -> Option<ExperienceSequenceId> {
        self.watermark.map(|watermark| watermark.sequence)
    }
    pub fn source_encoded_bytes(&self) -> Vec<usize> {
        self.records
            .iter()
            .map(|record| record.source_json.len())
            .collect()
    }

    /// Atomic admission. Missing measurement never creates a zero observation.
    /// Sources must arrive in sequence order with nonoverlapping interval clocks.
    pub fn observe(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<NeedConsequenceAdmissionV1, NeedConsequenceErrorV1> {
        // Bound source encoding before invoking deeper validation/cloning. This
        // also bounds fields whose legacy validators permit arbitrary vectors.
        let source_json =
            String::from_utf8(encode_bounded(patch, MAX_NEED_CONSEQUENCE_SOURCE_BYTES)?)
                .map_err(|_| NeedConsequenceErrorV1::MalformedSource)?;
        patch.validate_contract()?;
        if patch.header().organism_id != self.organism_id {
            return Err(NeedConsequenceErrorV1::ForeignOrganism);
        }
        let watermark = SourceWatermark {
            sequence: patch.header().sequence_id,
            before_tick: patch.pre_action().tick,
            after_tick: patch.outcome().outcome_tick,
        };
        check_next(self.watermark, watermark)?;
        let record = record_from_patch(patch, source_json)?;
        let mut next = self.clone();
        next.watermark = Some(watermark);
        let mut pruned_records = 0;
        let admission = if let Some(record) = record {
            next.records.push(record);
            while next.records.len() > next.capacity {
                next.records.remove(0);
                pruned_records += 1;
            }
            NeedConsequenceAdmissionV1::Measured { pruned_records: 0 }
        } else {
            NeedConsequenceAdmissionV1::Unknown
        };
        // Metadata can grow even on Unknown admission. Evict oldest first for
        // either count or exact encoded-size pressure, never erase the watermark.
        loop {
            match next.to_json() {
                Ok(_) => break,
                Err(NeedConsequenceErrorV1::ByteLimit) if !next.records.is_empty() => {
                    next.records.remove(0);
                    pruned_records += 1;
                }
                Err(error) => return Err(error),
            }
        }
        *self = next;
        Ok(match admission {
            NeedConsequenceAdmissionV1::Measured { .. } => {
                NeedConsequenceAdmissionV1::Measured { pruned_records }
            }
            NeedConsequenceAdmissionV1::Unknown => NeedConsequenceAdmissionV1::Unknown,
        })
    }

    /// Scan newest sources first. Unknown may mean absent, stale, inexact, or
    /// outside the explicit scan budget; it never means a measured zero effect.
    pub fn recall(
        &self,
        query: &NeedConsequenceQueryV1,
        as_of_tick: Tick,
        max_age_ticks: u64,
        scan_limit: usize,
    ) -> Result<NeedConsequenceRecallV1, NeedConsequenceErrorV1> {
        if scan_limit == 0 || scan_limit > MAX_NEED_CONSEQUENCE_QUERY_SCAN {
            return Err(NeedConsequenceErrorV1::InvalidLimit);
        }
        if query.organism_id != self.organism_id {
            return Err(NeedConsequenceErrorV1::ForeignOrganism);
        }
        let mut measured = Vec::new();
        let mut inspected_records = 0;
        for record in self.records.iter().rev().take(scan_limit) {
            inspected_records += 1;
            let observation = &record.observation;
            if let Some(age) = as_of_tick.raw().checked_sub(observation.after_tick.raw()) {
                if age <= max_age_ticks && &record.query == query {
                    measured.push(observation.clone());
                    if measured.len() == MAX_NEED_CONSEQUENCE_QUERY_RESULTS {
                        break;
                    }
                }
            }
        }
        Ok(NeedConsequenceRecallV1 {
            evidence: if measured.is_empty() {
                NeedConsequenceEvidenceV1::Unknown
            } else {
                NeedConsequenceEvidenceV1::Measured(measured)
            },
            inspected_records,
            budget_exhausted: inspected_records < self.records.len(),
        })
    }

    /// A bounded JSON envelope. Do not publish private acquired source banks.
    pub fn to_json(&self) -> Result<Vec<u8>, NeedConsequenceErrorV1> {
        let mut wire = StoreWire {
            schema_version: NEED_CONSEQUENCE_SCHEMA_VERSION,
            organism_id: self.organism_id,
            capacity: self.capacity,
            watermark: self.watermark,
            sources: self
                .records
                .iter()
                .map(|record| record.source_json.clone())
                .collect(),
            integrity_digest: [0; 4],
        };
        wire.integrity_digest = wire_digest(&wire);
        encode_bounded(&wire, MAX_NEED_CONSEQUENCE_STORE_BYTES)
    }

    /// Reject oversized input before parsing; no permissive generic Deserialize
    /// entrypoint and no migration from old memory records or experience ABIs.
    pub fn from_json(bytes: &[u8]) -> Result<Self, NeedConsequenceErrorV1> {
        if bytes.len() > MAX_NEED_CONSEQUENCE_STORE_BYTES {
            return Err(NeedConsequenceErrorV1::ByteLimit);
        }
        let wire: StoreWire =
            serde_json::from_slice(bytes).map_err(|_| NeedConsequenceErrorV1::MalformedSource)?;
        if wire.schema_version != NEED_CONSEQUENCE_SCHEMA_VERSION
            || wire.sources.len() > wire.capacity
            || wire.sources.len() > MAX_NEED_CONSEQUENCE_RECORDS
            || wire.integrity_digest != wire_digest(&wire)
        {
            return Err(NeedConsequenceErrorV1::MalformedSource);
        }
        let mut result = Self::new(wire.organism_id, wire.capacity)?;
        let mut previous = None;
        for source_json in wire.sources {
            if source_json.len() > MAX_NEED_CONSEQUENCE_SOURCE_BYTES {
                return Err(NeedConsequenceErrorV1::ByteLimit);
            }
            // ExperiencePatch has legacy migration branches. Explicitly reject
            // those ABIs before calling its parser, rather than importing them.
            let value: serde_json::Value = serde_json::from_str(&source_json)
                .map_err(|_| NeedConsequenceErrorV1::MalformedSource)?;
            let abi = value["header"]["abi_version"].as_u64();
            if ![
                ExperiencePatch::ABI_VERSION,
                ExperiencePatch::V11_ABI_VERSION,
                ExperiencePatch::V12_ABI_VERSION,
            ]
            .into_iter()
            .any(|allowed| abi == Some(u64::from(allowed)))
            {
                return Err(NeedConsequenceErrorV1::MalformedSource);
            }
            let patch: ExperiencePatch = serde_json::from_str(&source_json)
                .map_err(|_| NeedConsequenceErrorV1::MalformedSource)?;
            patch.validate_contract()?;
            // Sources emitted here have one canonical current representation.
            // Reject ignored/duplicate fields and fallback defaults on restore.
            if encode_bounded(&patch, MAX_NEED_CONSEQUENCE_SOURCE_BYTES)? != source_json.as_bytes()
            {
                return Err(NeedConsequenceErrorV1::MalformedSource);
            }
            if patch.header().organism_id != result.organism_id {
                return Err(NeedConsequenceErrorV1::ForeignOrganism);
            }
            let next = SourceWatermark {
                sequence: patch.header().sequence_id,
                before_tick: patch.pre_action().tick,
                after_tick: patch.outcome().outcome_tick,
            };
            check_next(previous, next)?;
            previous = Some(next);
            let record = record_from_patch(&patch, source_json)?
                .ok_or(NeedConsequenceErrorV1::MalformedSource)?;
            result.records.push(record);
        }
        if let Some(watermark) = wire.watermark {
            watermark.sequence.validate()?;
            Tick::validate_monotonic(watermark.before_tick, watermark.after_tick)?;
            if let Some(last) = previous {
                if watermark != last {
                    check_next(Some(last), watermark)?;
                }
            }
        } else if previous.is_some() {
            return Err(NeedConsequenceErrorV1::MalformedSource);
        }
        result.watermark = wire.watermark;
        // Also enforce canonical re-encoding size, independent of input spacing.
        result.to_json()?;
        Ok(result)
    }
}

fn check_next(
    previous: Option<SourceWatermark>,
    next: SourceWatermark,
) -> Result<(), NeedConsequenceErrorV1> {
    next.sequence.validate()?;
    Tick::validate_monotonic(next.before_tick, next.after_tick)?;
    if let Some(previous) = previous {
        if next.sequence.raw() <= previous.sequence.raw() {
            return Err(NeedConsequenceErrorV1::ReplayedSource);
        }
        if next.before_tick.raw() < previous.after_tick.raw() {
            return Err(NeedConsequenceErrorV1::BackwardInterval);
        }
    }
    Ok(())
}

fn record_from_patch(
    patch: &ExperiencePatch,
    source_json: String,
) -> Result<Option<Record>, NeedConsequenceErrorV1> {
    let Some(measured) = patch.outcome().measured_physiology.as_ref() else {
        return Ok(None);
    };
    measured.validate_contract()?;
    // Stronger local binding than the legacy outcome-only physiology check.
    let pre = patch.pre_action();
    if measured.before.source_genome_id != pre.genome_id
        || measured.before.tick != pre.tick
        || measured.before.homeostasis != *pre.homeostasis()
        || measured.after.tick != patch.outcome().outcome_tick
    {
        return Err(NeedConsequenceErrorV1::MalformedSource);
    }
    if pre.evidence_kind() == EvidenceKind::HeuristicBaseline {
        return Ok(None);
    }
    let decision = patch.decision();
    let evidence = decision.neural_evidence()?;
    let frame = pre.perception();
    let candidate = frame
        .candidates()
        .get(usize::from(evidence.candidate_index))
        .ok_or(NeedConsequenceErrorV1::MalformedSource)?;
    let query = NeedConsequenceQueryV1::from_frame(frame, candidate)?;
    if query.tracked_target.is_none() || query.entity.is_none() {
        return Ok(None);
    }
    // The source is associated only with the representative observed candidate.
    // A joint interval is never expanded into one full delta per motor channel.
    if let Some(bundle) = patch.selected_bundle() {
        let matching = bundle
            .channels
            .iter()
            .filter(|channel| {
                channel.primitive == candidate.action_id
                    && channel.target.and_then(|target| target.entity) == candidate.target.entity
            })
            .count();
        if matching != 1 {
            return Ok(None);
        }
    }
    let before = measured.before.homeostasis.drives.to_array();
    let after = measured.after.homeostasis.drives.to_array();
    let mut drive_deltas = [0.0; NEED_CONSEQUENCE_DRIVE_COUNT];
    for (i, delta) in drive_deltas.iter_mut().enumerate() {
        *delta = after[i] - before[i];
    }
    let observation = NeedConsequenceObservationV1 {
        source_sequence_id: patch.header().sequence_id,
        before_tick: measured.before.tick,
        after_tick: measured.after.tick,
        drive_deltas,
        confidence: Confidence::new(
            decision
                .confidence
                .raw()
                .min(candidate.sensor_confidence.raw()),
        )?,
        attribution: NeedConsequenceAttributionV1::JointIntervalAssociation,
        joint_command_count: patch
            .selected_bundle()
            .map_or(1, |bundle| bundle.channels.len()),
        source_frame_digest: frame.frame_digest().0,
        source_bundle_digest: patch
            .selected_bundle()
            .map(|bundle| bundle.canonical_digest())
            .transpose()?,
    };
    Ok(Some(Record {
        source_json,
        query,
        observation,
    }))
}

fn wire_digest(wire: &StoreWire) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"ALIFE-NEED-CONSEQUENCE-STORE-V1");
    digest.write_u16(wire.schema_version);
    digest.write_u64(wire.organism_id.raw());
    digest.write_u64(wire.capacity as u64);
    if let Some(watermark) = wire.watermark {
        digest.write_some();
        digest.write_u64(watermark.sequence.raw());
        digest.write_u64(watermark.before_tick.raw());
        digest.write_u64(watermark.after_tick.raw());
    } else {
        digest.write_none();
    }
    digest.write_sequence_len(wire.sources.len());
    for source in &wire.sources {
        digest.write_bytes(source.as_bytes());
    }
    digest.finish256()
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("need-consequence encoded byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encode_bounded(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, NeedConsequenceErrorV1> {
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|error| {
        if error.is_io() {
            NeedConsequenceErrorV1::ByteLimit
        } else {
            NeedConsequenceErrorV1::MalformedSource
        }
    })?;
    Ok(writer.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::ChannelPhysicalOutcome;
    use crate::*;

    #[derive(Clone)]
    struct Fixture {
        sequence: u64,
        tick: u64,
        organism: OrganismId,
        tracked: TrackedObjectId,
        entity: WorldEntityId,
        profile: SensorProfile,
        family: CandidateActionFamily,
        position: Vec3f,
        loneliness: f32,
        measured: bool,
        observed: bool,
        deltas: [f32; 11],
        padding: usize,
        utterance: u64,
    }
    impl Default for Fixture {
        fn default() -> Self {
            Self {
                sequence: 1,
                tick: 10,
                organism: OrganismId(7),
                tracked: TrackedObjectId(71),
                entity: WorldEntityId(88),
                profile: SensorProfile::GroundedObjectSlotsV1,
                family: CandidateActionFamily::Inspect,
                position: Vec3f::ZERO,
                loneliness: 0.5,
                measured: true,
                observed: true,
                deltas: [0.; 11],
                padding: 0,
                utterance: 0,
            }
        }
    }

    fn fixture(spec: Fixture) -> ExperiencePatch {
        let genome = CreatureGenome::early_mammal_founder(
            301,
            FoundationGeneticIdentity::new(22, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let tick = Tick(spec.tick);
        // Keep developmental age fixed so clock-only fixtures really retain the same hormones.
        let mut before = BiochemistryState::new_with_age(&phenotype, tick, Tick::ZERO).unwrap();
        before.homeostasis.drives = DriveSnapshot {
            hunger: 0.5,
            fatigue: 0.5,
            fear: 0.5,
            pain: 0.5,
            loneliness: spec.loneliness,
            curiosity: 0.5,
            brain_atp: 0.5,
            temperature_stress: 0.5,
            reproductive_drive: 0.5,
            extension: [0.5, 0.5],
        };
        let mut after =
            BiochemistryState::new_with_age(&phenotype, Tick(spec.tick + 1), Tick::ZERO).unwrap();
        after.homeostasis = before.homeostasis;
        after.homeostasis.tick = after.tick;
        let b = before.homeostasis.drives.to_array();
        let d = spec.deltas;
        after.homeostasis.drives = DriveSnapshot {
            hunger: b[0] + d[0],
            fatigue: b[1] + d[1],
            fear: b[2] + d[2],
            pain: b[3] + d[3],
            loneliness: b[4] + d[4],
            curiosity: b[5] + d[5],
            brain_atp: b[6] + d[6],
            temperature_stress: b[7] + d[7],
            reproductive_drive: b[8] + d[8],
            extension: [b[9] + d[9], b[10] + d[10]],
        };
        let slot = GroundedObjectSlotV1 {
            slot_index: 0,
            tracked_object_id: spec.tracked,
            bearing: [0., 0.],
            distance: 0.1,
            relative_velocity: [0.; 3],
            color: [0.2; 3],
            material: [0.3; 3],
            shape: [0.4; 3],
            chemical: [0.; 3],
            contact: 0.,
            proprioception: [0.; 2],
            temperature: 0.,
            terrain: [0.; 2],
            confidence: Confidence::new(0.8).unwrap(),
        };
        let (kind, action) = match spec.family {
            CandidateActionFamily::Contact => (ActionKind::Interact, ActionId(201)),
            CandidateActionFamily::Rest => (ActionKind::Rest, ActionId(2)),
            _ => (ActionKind::Inspect, ActionId(4)),
        };
        let candidate = ActionCandidate::new(
            0,
            action,
            kind,
            spec.family,
            if spec.observed {
                CandidateObservationRef::ObjectSlot(0)
            } else {
                CandidateObservationRef::None
            },
            if spec.family == CandidateActionFamily::Rest {
                ActionTarget::NONE
            } else {
                ActionTarget::new(Some(spec.entity), Some(Vec3f::new(1., 0., 0.)))
            },
            if spec.observed {
                slot.candidate_features().unwrap()
            } else {
                CandidateFeatureVector::zero()
            },
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.1).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap();
        let mut sensory = SensorySnapshot::new(
            spec.organism,
            tick,
            spec.position,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap();
        if spec.utterance != 0 {
            sensory.language_context.heard_tokens[0] = Some(HeardToken {
                utterance_id: UtteranceId::new(spec.utterance).unwrap(),
                sequence_position: 0,
                source_kind: UtteranceSourceKind::Creature,
                speaker_id: Some(OrganismId(8)),
                addressee: Some(spec.organism),
                source_entity: Some(spec.entity),
                token_id: 7,
                source_position: Vec3f::new(1., 0., 0.),
                confidence: Confidence::new(0.8).unwrap(),
                teacher_channel: None,
            });
            sensory.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
                agent_id: OrganismId(8),
                body_entity: Some(spec.entity),
                relative_position: Vec3f::new(1., 0., 0.),
                gaze_direction: Vec3f::new(-1., 0., 0.),
                orientation_forward: Vec3f::new(-1., 0., 0.),
                affinity: SignedValence::ZERO,
                proximity: NormalizedScalar::new(0.5).unwrap(),
            });
            sensory.semantic_context = Some(SemanticContextRef {
                feature_flags: ContextFeatureFlags::SEMANTIC_CODES,
                confidence: Confidence::new(0.8).unwrap(),
                compressed_codes: vec![CompressedSemanticCode {
                    codebook_id: 1,
                    code: 7,
                    salience: NormalizedScalar::new(0.5).unwrap(),
                }],
                salience: vec![SemanticSalienceEntry {
                    concept_id: ConceptCellId(1),
                    salience: NormalizedScalar::new(0.5).unwrap(),
                }],
            });
        }
        let frame = PerceptionFrame::new(
            spec.organism,
            tick,
            spec.profile,
            sensory,
            BodySnapshot {
                pose: Pose {
                    translation: spec.position,
                    ..Pose::IDENTITY
                },
                velocity: Velocity::ZERO,
            },
            before.homeostasis,
            vec![candidate],
            SensorProfileProvenance::new(spec.profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
            if spec.observed { vec![slot] } else { vec![] },
        )
        .unwrap();
        frame.validate_contract().unwrap();
        let sequence = ExperienceSequenceId(spec.sequence);
        let hash = PhenotypeHash([1, 2, 3, 4]);
        let decision = DecisionSnapshot::from_cpu_neural_selection(
            sequence,
            hash,
            spec.sequence,
            0,
            &frame,
            NeuralActionSelection {
                candidate_index: 0,
                logit: 0.,
                confidence: Confidence::new(0.9).unwrap(),
                active_tiles: 0,
                active_synapses: 0,
            },
            candidate
                .to_command(spec.organism, Confidence::new(0.9).unwrap())
                .unwrap(),
        )
        .unwrap();
        let pre = PreActionSnapshot::from_cpu_neural_frame(
            sequence,
            BrainClassSpec::for_tier(BrainScaleTier::Nano512).id,
            hash,
            phenotype.brain_genome.id,
            phenotype.brain_genome.schema_version,
            phenotype.development_state_at(tick).unwrap(),
            frame,
        )
        .unwrap();
        let mut outcome = PostActionOutcome::new(
            spec.organism,
            sequence,
            after.tick,
            true,
            PhysicalActionOutcome {
                contact: PhysicalContactKind::None,
                target_entity: Some(spec.entity),
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::ZERO,
            },
            HomeostaticDelta::zero(),
            SignedValence::new(0.9).unwrap(),
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap();
        if spec.measured {
            outcome = outcome
                .with_measured_physiology(MeasuredPhysiologyTransition::new(before, after).unwrap())
                .unwrap();
        }
        outcome.memory_hints = vec![
            MemoryHint {
                memory_id: MemoryId(1),
                salience: NormalizedScalar::ZERO
            };
            spec.padding
        ];
        ExperiencePatchBuilder::new(sequence)
            .record_pre_action(pre)
            .unwrap()
            .record_decision(decision)
            .unwrap()
            .record_outcome(outcome)
            .unwrap()
            .seal()
            .unwrap()
    }
    fn query(patch: &ExperiencePatch) -> NeedConsequenceQueryV1 {
        let frame = patch.pre_action().perception();
        NeedConsequenceQueryV1::from_frame(frame, &frame.candidates()[0]).unwrap()
    }
    fn measured(
        store: &NeedConsequenceStoreV1,
        patch: &ExperiencePatch,
    ) -> Vec<NeedConsequenceObservationV1> {
        match store
            .recall(&query(patch), Tick(1_000), 1_000, 32)
            .unwrap()
            .evidence
        {
            NeedConsequenceEvidenceV1::Measured(records) => records,
            NeedConsequenceEvidenceV1::Unknown => panic!("expected measured observation"),
        }
    }
    fn rewrite_store(bytes: &[u8], edit: impl FnOnce(&mut StoreWire)) -> Vec<u8> {
        let mut wire: StoreWire = serde_json::from_slice(bytes).unwrap();
        edit(&mut wire);
        wire.integrity_digest = wire_digest(&wire);
        serde_json::to_vec(&wire).unwrap()
    }
    fn rewrite_source(wire: &mut StoreWire, edit: impl FnOnce(&mut serde_json::Value)) {
        let mut source = serde_json::from_str(&wire.sources[0]).unwrap();
        edit(&mut source);
        wire.sources[0] = serde_json::to_string(&source).unwrap();
    }

    #[test]
    fn need_consequence_unknown_is_not_measured_zero_and_cannot_be_reinterpreted() {
        let absent = fixture(Fixture {
            measured: false,
            ..Fixture::default()
        });
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        assert_eq!(
            store.observe(&absent).unwrap(),
            NeedConsequenceAdmissionV1::Unknown
        );
        assert!(store.is_empty());
        assert_eq!(
            store
                .recall(&query(&absent), Tick(12), 100, 4)
                .unwrap()
                .evidence,
            NeedConsequenceEvidenceV1::Unknown
        );
        assert!(matches!(
            store.observe(&fixture(Fixture::default())),
            Err(NeedConsequenceErrorV1::ReplayedSource)
        ));
        let zero = fixture(Fixture {
            sequence: 2,
            tick: 12,
            ..Fixture::default()
        });
        store.observe(&zero).unwrap();
        let observed = measured(&store, &zero);
        assert_eq!(observed[0].drive_deltas, [0.; 11]);
        assert_eq!(observed[0].confidence.raw(), 0.8);
        assert_eq!(
            observed[0].attribution,
            NeedConsequenceAttributionV1::JointIntervalAssociation
        );
    }

    #[test]
    fn need_consequence_preserves_all_signed_lanes_and_source_identity() {
        let deltas = [
            -0.125, 0.125, -0.25, 0.25, -0.125, 0.125, 0.25, -0.25, 0.125, -0.125, 0.25,
        ];
        let patch = fixture(Fixture {
            deltas,
            ..Fixture::default()
        });
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&patch).unwrap();
        let observations = measured(&store, &patch);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].drive_deltas, deltas);
        assert_eq!(observations[0].source_sequence_id, ExperienceSequenceId(1));
        assert_eq!(observations[0].before_tick, Tick(10));
        assert_eq!(observations[0].after_tick, Tick(11));
        assert_eq!(observations[0].joint_command_count, 1);
        assert!(observations[0].source_bundle_digest.is_none());
        assert_eq!(
            observations[0].source_frame_digest,
            patch.pre_action().perception().frame_digest().0
        );
    }

    #[test]
    fn need_consequence_exact_target_family_profile_context_and_later_clock() {
        let patch = fixture(Fixture::default());
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&patch).unwrap();
        let later = fixture(Fixture {
            sequence: 2,
            tick: 40,
            ..Fixture::default()
        });
        assert_eq!(query(&patch), query(&later));
        assert_eq!(measured(&store, &later).len(), 1);
        for changed in [
            Fixture {
                tracked: TrackedObjectId(72),
                ..Fixture::default()
            },
            Fixture {
                entity: WorldEntityId(89),
                ..Fixture::default()
            },
            Fixture {
                family: CandidateActionFamily::Contact,
                ..Fixture::default()
            },
            Fixture {
                profile: SensorProfile::GroundedTerrainVisionV1,
                ..Fixture::default()
            },
            Fixture {
                loneliness: 0.75,
                ..Fixture::default()
            },
            Fixture {
                position: Vec3f::new(1., 0., 0.),
                ..Fixture::default()
            },
        ] {
            assert_eq!(
                store
                    .recall(&query(&fixture(changed)), Tick(50), 100, 4)
                    .unwrap()
                    .evidence,
                NeedConsequenceEvidenceV1::Unknown
            );
        }
        let foreign = fixture(Fixture {
            organism: OrganismId(8),
            ..Fixture::default()
        });
        assert!(matches!(
            store.recall(&query(&foreign), Tick(50), 100, 4),
            Err(NeedConsequenceErrorV1::ForeignOrganism)
        ));
        assert!(matches!(
            store.observe(&foreign),
            Err(NeedConsequenceErrorV1::ForeignOrganism)
        ));
        let mut changed_candidate = patch.pre_action().perception().candidates()[0];
        changed_candidate.target.entity = Some(WorldEntityId(999));
        assert!(NeedConsequenceQueryV1::from_frame(
            patch.pre_action().perception(),
            &changed_candidate
        )
        .is_err());
    }

    #[test]
    fn need_consequence_unobserved_target_does_not_create_evidence() {
        let patch = fixture(Fixture {
            observed: false,
            family: CandidateActionFamily::Rest,
            ..Fixture::default()
        });
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        assert_eq!(
            store.observe(&patch).unwrap(),
            NeedConsequenceAdmissionV1::Unknown
        );
        assert!(store.is_empty());
        assert_eq!(store.last_source_sequence(), Some(ExperienceSequenceId(1)));
    }

    #[test]
    fn need_consequence_pruning_and_restore_preserve_monotonic_replay_guard() {
        let mut a = NeedConsequenceStoreV1::new(OrganismId(7), 2).unwrap();
        let mut b = a.clone();
        for sequence in 1..=5 {
            let patch = fixture(Fixture {
                sequence,
                tick: sequence * 2,
                ..Fixture::default()
            });
            a.observe(&patch).unwrap();
            b.observe(&patch).unwrap();
        }
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
        assert_eq!(a.len(), 2);
        let saved = a.to_json().unwrap();
        let mut restored = NeedConsequenceStoreV1::from_json(&saved).unwrap();
        assert_eq!(restored, a);
        assert_eq!(restored.to_json().unwrap(), saved);
        for sequence in [1, 4, 5] {
            let old = fixture(Fixture {
                sequence,
                tick: sequence * 2,
                ..Fixture::default()
            });
            assert!(matches!(
                restored.observe(&old),
                Err(NeedConsequenceErrorV1::ReplayedSource)
            ));
        }
        let rewind = fixture(Fixture {
            sequence: 6,
            tick: 9,
            ..Fixture::default()
        });
        assert!(matches!(
            restored.observe(&rewind),
            Err(NeedConsequenceErrorV1::BackwardInterval)
        ));
        assert_eq!(restored.to_json().unwrap(), saved);
        let unknown = fixture(Fixture {
            sequence: 6,
            tick: 12,
            measured: false,
            ..Fixture::default()
        });
        restored.observe(&unknown).unwrap();
        let mut restored = NeedConsequenceStoreV1::from_json(&restored.to_json().unwrap()).unwrap();
        assert!(matches!(
            restored.observe(&unknown),
            Err(NeedConsequenceErrorV1::ReplayedSource)
        ));
    }

    #[test]
    fn need_consequence_recall_age_scan_and_output_are_bounded() {
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 16).unwrap();
        for sequence in 1..=12 {
            store
                .observe(&fixture(Fixture {
                    sequence,
                    tick: sequence * 2,
                    ..Fixture::default()
                }))
                .unwrap();
        }
        let q = query(&fixture(Fixture::default()));
        let short = store.recall(&q, Tick(30), 100, 3).unwrap();
        assert_eq!(short.inspected_records, 3);
        assert!(short.budget_exhausted);
        let full = store.recall(&q, Tick(30), 100, 32).unwrap();
        let NeedConsequenceEvidenceV1::Measured(values) = full.evidence else {
            panic!()
        };
        assert_eq!(values.len(), 8);
        assert_eq!(values[0].source_sequence_id, ExperienceSequenceId(12));
        assert_eq!(full.inspected_records, 8);
        assert!(full.budget_exhausted);
        assert_eq!(
            store.recall(&q, Tick(30), 1, 32).unwrap().evidence,
            NeedConsequenceEvidenceV1::Unknown
        );
        assert_eq!(
            store.recall(&q, Tick(1), 100, 32).unwrap().evidence,
            NeedConsequenceEvidenceV1::Unknown
        );
        for invalid in [0, 33, usize::MAX] {
            assert!(store.recall(&q, Tick(30), 100, invalid).is_err());
        }
        for invalid in [0, 33, usize::MAX] {
            assert!(NeedConsequenceStoreV1::new(OrganismId(7), invalid).is_err());
        }
    }

    #[test]
    fn need_consequence_byte_caps_and_size_eviction_are_enforced() {
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 32).unwrap();
        let patch = fixture(Fixture::default());
        store.observe(&patch).unwrap();
        println!(
            "need-consequence synthetic source bytes={} one-record store bytes={}",
            store.source_encoded_bytes()[0],
            store.to_json().unwrap().len()
        );
        let saved = store.to_json().unwrap();
        let huge = fixture(Fixture {
            sequence: 2,
            tick: 12,
            padding: 6_000,
            ..Fixture::default()
        });
        assert!(matches!(
            store.observe(&huge),
            Err(NeedConsequenceErrorV1::ByteLimit)
        ));
        assert_eq!(store.to_json().unwrap(), saved);
        assert!(matches!(
            NeedConsequenceStoreV1::from_json(&vec![b' '; MAX_NEED_CONSEQUENCE_STORE_BYTES + 1]),
            Err(NeedConsequenceErrorV1::ByteLimit)
        ));
        for sequence in 2..=32 {
            store
                .observe(&fixture(Fixture {
                    sequence,
                    tick: sequence * 2 + 10,
                    padding: 1_500,
                    ..Fixture::default()
                }))
                .unwrap();
            assert!(store.to_json().unwrap().len() <= MAX_NEED_CONSEQUENCE_STORE_BYTES);
        }
        assert!(
            store.len() < 32,
            "byte pressure must prune before count pressure"
        );
        assert_eq!(store.last_source_sequence(), Some(ExperienceSequenceId(32)));
        assert!(store
            .source_encoded_bytes()
            .iter()
            .all(|bytes| *bytes <= MAX_NEED_CONSEQUENCE_SOURCE_BYTES));
        println!(
            "need-consequence padded sources={} encoded store bytes={}",
            store.len(),
            store.to_json().unwrap().len()
        );
        NeedConsequenceStoreV1::from_json(&store.to_json().unwrap()).unwrap();
    }

    #[test]
    fn need_consequence_restore_revalidates_sources_even_with_recomputed_envelope_digest() {
        let patch = fixture(Fixture::default());
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&patch).unwrap();
        let saved = store.to_json().unwrap();
        for malformed in [
            rewrite_store(&saved, |wire| wire.schema_version = 2),
            rewrite_store(&saved, |wire| wire.watermark = None),
            rewrite_store(&saved, |wire| wire.sources.push(wire.sources[0].clone())),
            rewrite_store(&saved, |wire| wire.organism_id = OrganismId(8)),
            rewrite_store(&saved, |wire| {
                rewrite_source(wire, |source| source["header"]["abi_version"] = 1.into())
            }),
            rewrite_store(&saved, |wire| {
                rewrite_source(wire, |source| {
                    source["outcome"]["measured_physiology"] = serde_json::Value::Null
                })
            }),
            rewrite_store(&saved, |wire| {
                rewrite_source(wire, |source| {
                    source["outcome"]["measured_physiology"]["homeostatic_delta"]["drives"]
                        ["loneliness"] = 0.25.into()
                })
            }),
            rewrite_store(&saved, |wire| {
                rewrite_source(wire, |source| {
                    source["outcome"]["measured_physiology"]["before"]["tick"] = 9.into()
                })
            }),
            rewrite_store(&saved, |wire| {
                rewrite_source(wire, |source| {
                    source["outcome"]["measured_physiology"]["before"]["homeostasis"]["drives"]
                        ["loneliness"] = 2.into()
                })
            }),
        ] {
            assert!(NeedConsequenceStoreV1::from_json(&malformed).is_err());
        }
        let mut corrupt: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        corrupt["watermark"]["sequence"] = 99.into();
        assert!(NeedConsequenceStoreV1::from_json(&serde_json::to_vec(&corrupt).unwrap()).is_err());
        for malformed in [b"{}".as_slice(), &saved[..saved.len() - 1], b"null"] {
            assert!(NeedConsequenceStoreV1::from_json(malformed).is_err());
        }
        // Old V2 evidence cannot be implicitly imported into this new schema.
        assert!(
            NeedConsequenceStoreV1::from_json(br#"{"schema_version":2,"records":[]}"#).is_err()
        );
    }

    fn joint_fixture(mode: u8) -> ExperiencePatch {
        let mut spec = Fixture::default();
        spec.deltas[4] = -0.125;
        let base = fixture(spec);
        let pre = base.pre_action();
        let candidate = pre.perception().candidates()[0];
        let first_target = if mode == 1 {
            ActionTarget::new(Some(WorldEntityId(999)), candidate.target.position)
        } else {
            candidate.target
        };
        let first = ChannelCommand::new(
            MotorChannel::Orientation,
            candidate.action_id,
            Some(first_target),
            Vec3f::ZERO,
            Intensity::new(1.).unwrap(),
            DurationTicks::new(1),
            0.,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let second = ChannelCommand::new(
            MotorChannel::Posture,
            if mode == 2 {
                candidate.action_id
            } else {
                ActionId(999)
            },
            if mode == 2 {
                Some(candidate.target)
            } else {
                None
            },
            Vec3f::ZERO,
            Intensity::new(1.).unwrap(),
            DurationTicks::new(1),
            0.,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let bundle = MotorCommandBundle::new(
            pre.organism_id,
            pre.sequence_id,
            pre.tick,
            vec![first, second],
        )
        .unwrap();
        let work = CognitiveWorkReceipt::from_counters(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0).unwrap();
        let prediction = PredictionTargetReceipt::for_successor(
            pre.organism_id,
            pre.sequence_id,
            candidate.action_id,
            pre.tick,
            [1, 2, 3, 4],
            SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
            JointMotorCondition::from_bundle(&bundle).unwrap(),
            SemanticStateVector::new(vec![0.2, 0.8]).unwrap(),
        )
        .unwrap();
        let physical = base.outcome().physical;
        let joint = JointPhysicalOutcome::new(physical, vec![])
            .unwrap()
            .with_channel_outcomes(
                bundle
                    .channels
                    .iter()
                    .map(|command| {
                        ChannelPhysicalOutcome::new(
                            command.channel,
                            PhysicalActionOutcome {
                                target_entity: command.target.and_then(|target| target.entity),
                                ..physical
                            },
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap();
        ExperiencePatch::new_v12_with_decision(
            pre.clone(),
            base.decision().clone(),
            bundle,
            base.outcome().clone().with_v12_joint(joint, work).unwrap(),
            prediction,
            work,
            CognitiveContextFrame::empty(pre.organism_id, pre.sequence_id, pre.tick).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn need_consequence_joint_source_is_one_association_and_ambiguity_stays_unknown() {
        let patch = joint_fixture(0);
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&patch).unwrap();
        let records = measured(&store, &patch);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].joint_command_count, 2);
        assert_eq!(records[0].drive_deltas[4], -0.125);
        assert_eq!(
            records[0].attribution,
            NeedConsequenceAttributionV1::JointIntervalAssociation
        );
        assert_eq!(
            records[0].source_bundle_digest,
            Some(patch.selected_bundle().unwrap().canonical_digest().unwrap())
        );
        let restored = NeedConsequenceStoreV1::from_json(&store.to_json().unwrap()).unwrap();
        assert_eq!(measured(&restored, &patch), records);
        for mode in [1, 2] {
            let patch = joint_fixture(mode);
            let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
            assert_eq!(
                store.observe(&patch).unwrap(),
                NeedConsequenceAdmissionV1::Unknown
            );
            assert!(store.is_empty());
            assert_eq!(store.last_source_sequence(), Some(ExperienceSequenceId(1)));
        }
    }

    #[test]
    fn need_consequence_opposite_observations_do_not_merge_into_zero() {
        let mut first = Fixture::default();
        first.deltas[4] = -0.125;
        let first = fixture(first);
        let mut second = Fixture {
            sequence: 2,
            tick: 12,
            ..Fixture::default()
        };
        second.deltas[4] = 0.125;
        let second = fixture(second);
        assert_eq!(query(&first), query(&second));
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&first).unwrap();
        store.observe(&second).unwrap();
        let records = measured(&store, &first);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].drive_deltas[4], 0.125);
        assert_eq!(records[1].drive_deltas[4], -0.125);
        assert_ne!(records[0].source_sequence_id, records[1].source_sequence_id);
    }

    #[test]
    fn need_consequence_context_projection_preserves_nonzero_time_observation_payloads() {
        let first = fixture(Fixture {
            utterance: 123,
            ..Fixture::default()
        });
        let later = fixture(Fixture {
            sequence: 2,
            tick: 40,
            utterance: 123,
            ..Fixture::default()
        });
        first.validate_contract().unwrap();
        later.validate_contract().unwrap();
        assert_eq!(query(&first), query(&later));
        let mut store = NeedConsequenceStoreV1::new(OrganismId(7), 4).unwrap();
        store.observe(&first).unwrap();
        assert_eq!(measured(&store, &later).len(), 1);
        let changed_event = fixture(Fixture {
            sequence: 2,
            tick: 40,
            utterance: 124,
            ..Fixture::default()
        });
        assert_eq!(
            store
                .recall(&query(&changed_event), Tick(50), 100, 4)
                .unwrap()
                .evidence,
            NeedConsequenceEvidenceV1::Unknown
        );
        assert_eq!(first.pre_action().perception().sensory().tick, Tick(10));
        assert_eq!(later.pre_action().perception().sensory().tick, Tick(40));
        let restored = NeedConsequenceStoreV1::from_json(&store.to_json().unwrap()).unwrap();
        assert_eq!(measured(&restored, &later), measured(&store, &first));
    }
}
