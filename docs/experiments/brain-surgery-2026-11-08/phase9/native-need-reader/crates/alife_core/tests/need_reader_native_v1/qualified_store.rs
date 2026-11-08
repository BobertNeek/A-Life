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
